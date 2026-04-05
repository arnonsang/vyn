use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use anyhow::{Context, Result};
use console::style;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use vyn_core::relay_storage::RelayStorageProvider;

use crate::output;

pub fn run() -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;

    output::print_banner("auth");
    println!("  vyn uses GitHub to identify you and your SSH key to encrypt vault invites.");
    println!("  Both steps are needed before you can share or receive vaults.");
    println!();

    print_step(1, "Authenticate with GitHub");
    let username = resolve_github_username()?;
    output::print_info("GitHub account", &format!("@{username}"));

    println!();
    print_step(2, "Locate local SSH key");
    let ssh_private_key =
        detect_ssh_private_key().context("failed to detect local SSH private key")?;
    let ssh_public_key = to_public_key_path(&ssh_private_key);

    if !ssh_public_key.exists() {
        anyhow::bail!(
            "missing SSH public key for detected private key: {}",
            ssh_public_key.display()
        );
    }

    let pubkey_content = fs::read_to_string(&ssh_public_key)
        .with_context(|| format!("failed to read {}", ssh_public_key.display()))?;

    output::print_info("Private key", &ssh_private_key.display().to_string());
    output::print_info("Public key", &ssh_public_key.display().to_string());

    println!();
    print_step(3, "Verify SSH key is registered on GitHub");
    println!("  vyn encrypts vault invites for you using your GitHub-listed SSH key.");
    println!("  Your local key must match one of the keys at:");
    println!("    https://github.com/{username}.keys");
    println!();

    if let Err(e) = verify_github_identity(&username, &ssh_private_key) {
        // Key not on GitHub, print the key and guide the user
        println!("  {} {}", style("✗").red().bold(), e);
        println!();
        println!(
            "  {} Add your SSH public key to GitHub:",
            style("→").cyan().bold()
        );
        println!(
            "    1. Go to  {}",
            style("https://github.com/settings/keys")
                .cyan()
                .underlined()
        );
        println!("    2. Click  'New SSH key'");
        println!("    3. Paste the following key (select all, copy, paste into the 'Key' field):");
        println!();
        for line in pubkey_content.trim().lines() {
            println!("     {}", style(line).yellow());
        }
        println!();
        println!("  Then run  {} again.", style("vyn auth").cyan());
        println!();
        std::process::exit(1);
    }

    let identity = IdentityConfig {
        github_username: username.clone(),
        ssh_private_key: ssh_private_key.to_string_lossy().to_string(),
        ssh_public_key: ssh_public_key.to_string_lossy().to_string(),
    };

    let vault_dir = root.join(".vyn");
    fs::create_dir_all(&vault_dir).context("failed to create .vyn directory")?;
    let identity_path = vault_dir.join("identity.toml");
    fs::write(
        &identity_path,
        toml::to_string_pretty(&identity).context("failed to encode identity config")?,
    )
    .with_context(|| format!("failed to write {}", identity_path.display()))?;

    // Register identity on the relay if this vault is configured for relay storage
    let relay_registered = try_register_on_relay(&vault_dir);

    println!();
    output::print_success(&format!("authenticated as @{username}"));
    output::print_info("SSH key", &ssh_public_key.display().to_string());
    output::print_info("Identity file", &identity_path.display().to_string());
    if let Some(relay_url) = relay_registered {
        output::print_info("Relay", &format!("identity registered on {relay_url}"));
    }
    println!();
    println!(
        "  You are ready to use  {} and {}.",
        style("vyn share").cyan(),
        style("vyn link").cyan()
    );
    println!();
    Ok(())
}

#[derive(Debug, Deserialize)]
struct VaultConfig {
    storage_provider: String,
    relay_url: Option<String>,
}

fn try_register_on_relay(vault_dir: &Path) -> Option<String> {
    let config_path = vault_dir.join("config.toml");
    let config_text = fs::read_to_string(&config_path).ok()?;
    let config: VaultConfig = toml::from_str(&config_text).ok()?;
    if config.storage_provider != "relay" {
        return None;
    }
    let relay_url = config.relay_url?;

    let rt = tokio::runtime::Runtime::new().ok()?;
    let provider = RelayStorageProvider::new(relay_url.clone());
    let vault_dir = vault_dir.to_path_buf();
    let result = rt.block_on(provider.authenticate_with_identity(&vault_dir));
    if result.is_ok() {
        Some(relay_url)
    } else {
        None
    }
}

fn print_step(n: u8, label: &str) {
    println!(
        "  {} {}",
        style(format!("[{n}/3]")).cyan().bold(),
        style(label).bold(),
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub github_username: String,
    pub ssh_private_key: String,
    pub ssh_public_key: String,
}

/// GitHub OAuth App `client_id` (Device Flow)
/// `None` to fall back to the `VYN_GITHUB_CLIENT_ID` env / Some hardcoded default for convenience.
const BUILTIN_CLIENT_ID: Option<&str> = Some("Ov23lipOXQFBV3RzETQj");

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubUser {
    login: String,
}

fn resolve_github_username() -> Result<String> {
    try_oauth_device_flow_username()
}

fn try_oauth_device_flow_username() -> Result<String> {
    let client_id = BUILTIN_CLIENT_ID
        .map(|s| s.to_string())
        .or_else(|| std::env::var("VYN_GITHUB_CLIENT_ID").ok())
        .context("no GitHub OAuth client_id configured; set VYN_GITHUB_CLIENT_ID or register an OAuth App")?;
    let client = Client::builder()
        .user_agent("vyn-cli")
        .build()
        .context("failed to initialize HTTP client")?;

    let code = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", client_id.as_str()), ("scope", "read:user")])
        .send()
        .context("failed to start GitHub device flow")?
        .error_for_status()
        .context("GitHub device flow request failed")?
        .json::<DeviceCodeResponse>()
        .context("invalid GitHub device-code response")?;

    println!(
        "  {} Open the URL below and enter your one-time code:",
        style("→").cyan().bold()
    );
    println!();
    println!(
        "    URL   {}",
        style(&code.verification_uri).cyan().underlined()
    );
    println!("    Code  {}", style(&code.user_code).yellow().bold());
    println!();
    println!(
        "  Waiting for authorization… (code expires at {})",
        expiry_time(code.expires_in)
    );
    let _ = try_open_browser(&code.verification_uri);

    let max_wait = Duration::from_secs(code.expires_in);
    let mut poll_interval = Duration::from_secs(code.interval.max(1));
    let started = std::time::Instant::now();

    while started.elapsed() < max_wait {
        thread::sleep(poll_interval);

        let token = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id.as_str()),
                ("device_code", code.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .context("failed to poll GitHub device flow")?
            .error_for_status()
            .context("GitHub device-token request failed")?
            .json::<DeviceTokenResponse>()
            .context("invalid GitHub device-token response")?;

        if let Some(access_token) = token.access_token {
            let user = client
                .get("https://api.github.com/user")
                .bearer_auth(access_token)
                .header("Accept", "application/vnd.github+json")
                .send()
                .context("failed to fetch authenticated GitHub user")?
                .error_for_status()
                .context("GitHub user endpoint returned an error")?
                .json::<GithubUser>()
                .context("invalid GitHub user response")?;

            return Ok(user.login);
        }

        match token.error.as_deref() {
            Some("authorization_pending") => continue,
            // GitHub adds 5 s to the required interval on slow_down (RFC 8628 §3.5)
            Some("slow_down") => {
                poll_interval += Duration::from_secs(5);
                continue;
            }
            Some("expired_token") => {
                anyhow::bail!("Authorization timed out. Run `vyn auth` again to restart the flow.")
            }
            Some("access_denied") => {
                anyhow::bail!("Authorization was cancelled by the user.")
            }
            Some(other) => anyhow::bail!("GitHub device flow failed: {other}"),
            None => continue,
        }
    }

    anyhow::bail!("GitHub device flow timed out")
}

/// Format a wall-clock expiry time (now + `secs_from_now`) as "HH:MM:SS".
fn expiry_time(secs_from_now: u64) -> String {
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + secs_from_now;
    let h = (expires_at % 86400) / 3600;
    let m = (expires_at % 3600) / 60;
    let s = expires_at % 60;
    format!("{h:02}:{m:02}:{s:02} UTC")
}

fn try_open_browser(url: &str) -> Result<()> {
    let candidates = [
        ("xdg-open", vec![url.to_string()]),
        ("open", vec![url.to_string()]),
        (
            "cmd",
            vec!["/C".to_string(), "start".to_string(), url.to_string()],
        ),
    ];

    for (program, args) in candidates {
        let status = std::process::Command::new(program).args(&args).status();
        if let Ok(status) = status
            && status.success()
        {
            return Ok(());
        }
    }

    anyhow::bail!("unable to open browser automatically")
}

fn detect_ssh_private_key() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable is not set")?;
    let ssh_dir = Path::new(&home).join(".ssh");
    let candidates = ["id_ed25519", "id_rsa"];

    for candidate in candidates {
        let path = ssh_dir.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "no supported SSH private key found under {} (looked for id_ed25519 and id_rsa)",
        ssh_dir.display()
    )
}

fn to_public_key_path(private_key: &Path) -> PathBuf {
    PathBuf::from(format!("{}.pub", private_key.display()))
}

/// Verify that `username` is a real GitHub account, that the local SSH private key
/// corresponds to one of the keys registered on it, and that the user actually holds
/// the private key (challenge–response via `ssh-keygen -Y sign/verify`).
///
/// Skipped when `VYN_SKIP_GITHUB_VERIFY=1` is set (useful in offline CI).
fn verify_github_identity(username: &str, private_key_path: &Path) -> Result<()> {
    #[cfg(any(test, debug_assertions))]
    if std::env::var("VYN_SKIP_GITHUB_VERIFY").as_deref() == Ok("1") {
        return Ok(());
    }

    let client = Client::builder()
        .user_agent("vyn-cli")
        .build()
        .context("failed to initialize HTTP client")?;

    let url = format!("https://github.com/{username}.keys");
    let response = client
        .get(&url)
        .send()
        .with_context(|| format!("failed to reach GitHub to verify @{username}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!(
            "GitHub account '@{username}' does not exist. \
             Check the username and try again."
        );
    }

    response
        .error_for_status_ref()
        .with_context(|| format!("GitHub returned an error while verifying @{username}"))?;

    let body = response
        .text()
        .context("failed to read GitHub key response")?;

    let github_keys: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if github_keys.is_empty() {
        anyhow::bail!(
            "GitHub account '@{username}' has no SSH keys registered.\n\
             vyn uses your GitHub SSH key to encrypt vault invites for you.\n\
             Add your SSH public key at https://github.com/settings/keys and try again."
        );
    }

    let tmp_dir = std::env::temp_dir().join(format!("vyn-auth-{}", Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir).context("failed to create auth challenge temp dir")?;
    let result = run_ssh_ownership_challenge(username, private_key_path, &github_keys, &tmp_dir);
    let _ = fs::remove_dir_all(&tmp_dir);
    result
}

/// Sign a random challenge with the local key and verify the signature against
/// the set of keys GitHub has on file for `username`.  Succeeds only when:
///   
///  - the local private key matches one of `github_keys`, AND
///  - the user can produce a valid signature (i.e. they hold the private key).
fn run_ssh_ownership_challenge(
    username: &str,
    private_key_path: &Path,
    github_keys: &[&str],
    tmp_dir: &Path,
) -> Result<()> {
    let challenge_path = tmp_dir.join("challenge");
    let sig_path = tmp_dir.join("challenge.sig");
    let signers_path = tmp_dir.join("allowed_signers");

    // 2x UUIDs give 244 bits of randomness (getrandom-backed)
    let nonce = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    fs::write(&challenge_path, nonce.as_bytes()).context("failed to write auth challenge")?;

    println!(
        "  {} Signing a one-time challenge to prove key ownership{}",
        style("→").cyan().bold(),
        style(" (ssh-keygen may prompt for a passphrase)").dim(),
    );
    let sign_status = Command::new("ssh-keygen")
        .args([
            "-Y",
            "sign",
            "-f",
            private_key_path.to_str().context("non-UTF-8 key path")?,
            "-n",
            "vyn-auth",
            challenge_path.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit()) // keep stderr so passphrase prompt shows
        .status()
        .context("failed to run ssh-keygen (is it installed and on PATH?)")?;

    if !sign_status.success() {
        anyhow::bail!("SSH signing step failed: could not prove key ownership");
    }
    if !sig_path.exists() {
        anyhow::bail!(
            "ssh-keygen did not produce a signature file at {}",
            sig_path.display()
        );
    }

    // Build allowed_signers: one line per GitHub-registered key, all sharing the same principal.
    // Format: <principal> <keytype> <base64blob> [optional comment]
    let principal = format!("{}@github", username);
    let signers_content: String = github_keys
        .iter()
        .map(|k| format!("{} {}\n", principal, k))
        .collect();
    fs::write(&signers_path, &signers_content).context("failed to write allowed_signers")?;

    let challenge_file =
        fs::File::open(&challenge_path).context("failed to open challenge for verification")?;
    let verify_output = Command::new("ssh-keygen")
        .args([
            "-Y",
            "verify",
            "-f",
            signers_path.to_str().unwrap(),
            "-n",
            "vyn-auth",
            "-I",
            &principal,
            "-s",
            sig_path.to_str().unwrap(),
        ])
        .stdin(challenge_file)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run ssh-keygen for verification")?;

    if !verify_output.status.success() {
        let stderr = String::from_utf8_lossy(&verify_output.stderr);
        anyhow::bail!(
            "Your local SSH private key does not match any key registered on \
             GitHub account '@{username}'.\n\
             vyn uses your GitHub-listed SSH key to encrypt vault invites, so they \
             must match. Add your SSH public key at https://github.com/settings/keys \
             and try again.\n\
             Details: {}",
            stderr.trim()
        );
    }

    Ok(())
}
