use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use vyn_core::keychain::load_project_key;
use vyn_core::relay_storage::RelayStorageProvider;
use vyn_core::storage::StorageProvider;
use vyn_core::wrapping::wrap_project_key_for_ssh_recipient;

use crate::output;

#[derive(Debug, Deserialize)]
struct VaultConfig {
    vault_id: String,
    storage_provider: String,
    relay_url: Option<String>,
}

pub fn run(user: String) -> Result<()> {
    output::print_banner("share");
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let username = user.trim().trim_start_matches('@').to_string();
    if username.is_empty() {
        anyhow::bail!("username must not be empty");
    }

    let config = load_config(&root)?;
    let vault_id = config.vault_id.clone();
    let key = load_project_key(&vault_id).context("failed to load project key from keychain")?;

    let spinner = output::new_spinner(&format!("fetching SSH keys for @{username}…"));
    let public_keys = fetch_github_public_keys(&username)?;
    if public_keys.is_empty() {
        output::fail_progress(
            &spinner,
            &format!("no SSH public keys found for @{username}"),
        );
        anyhow::bail!("no SSH public keys found for @{username}");
    }
    output::finish_progress(&spinner, &format!("{} SSH key(s) found", public_keys.len()));

    let relay_url = config
        .relay_url
        .clone()
        .context("missing `relay_url` in .vyn/config.toml — run `vyn config` to set it")?;

    if config.storage_provider != "relay" {
        anyhow::bail!(
            "relay-based invite sharing requires `storage_provider = \"relay\"` in .vyn/config.toml"
        );
    }

    let vault_dir = root.join(".vyn");
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(async {
        let provider = RelayStorageProvider::new(relay_url);
        provider
            .authenticate_with_identity(&vault_dir)
            .await
            .context("relay authentication failed (run `vyn auth` first)")?;

        let spinner2 = output::new_spinner(&format!("uploading invite(s) for @{username}…"));
        let mut uploaded = 0usize;
        for public_key in &public_keys {
            match wrap_project_key_for_ssh_recipient(&key, public_key) {
                Ok(payload) => {
                    provider
                        .create_invite(&username, &vault_id, payload)
                        .await
                        .context("failed to upload invite to relay")?;
                    uploaded += 1;
                }
                Err(e) => {
                    eprintln!("warning: could not wrap key for one SSH key: {e}");
                }
            }
        }

        if uploaded == 0 {
            output::fail_progress(&spinner2, "no invites could be created");
            anyhow::bail!("unable to encrypt invites with any of @{username}'s SSH public keys");
        }
        output::finish_progress(&spinner2, &format!("{uploaded} invite(s) uploaded"));

        output::print_success(&format!("invite sent to @{username}"));
        output::print_info("vault id", &vault_id);
        output::print_info(
            "next step",
            &format!("@{username} can now run: vyn link {vault_id}"),
        );
        println!();

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn load_config(root: &Path) -> Result<VaultConfig> {
    let config_path = root.join(".vyn").join("config.toml");
    let config_text = fs::read_to_string(&config_path)
        .with_context(|| format!("missing or unreadable file: {}", config_path.display()))?;
    toml::from_str(&config_text).context("invalid .vyn/config.toml format")
}

fn fetch_github_public_keys(username: &str) -> Result<Vec<String>> {
    #[cfg(any(test, debug_assertions))]
    if let Ok(path) = std::env::var("VYN_SHARE_SSH_KEYS") {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("VYN_SHARE_SSH_KEYS: failed to read {path}"))?;
        return Ok(parse_github_keys_response(&content));
    }
    fetch_github_public_keys_from_base(username, "https://github.com")
}

fn fetch_github_public_keys_from_base(username: &str, base_url: &str) -> Result<Vec<String>> {
    let url = format!("{base_url}/{username}.keys");
    let client = Client::builder()
        .build()
        .context("failed to initialize HTTP client")?;
    let body = client
        .get(url)
        .send()
        .context("failed to request GitHub public keys")?
        .error_for_status()
        .context("GitHub key endpoint returned an error")?
        .text()
        .context("failed to read GitHub key response body")?;

    Ok(parse_github_keys_response(&body))
}

fn parse_github_keys_response(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fetch_github_public_keys_from_base;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn github_key_fetch() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("local addr should be available");

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 2048];
                let _ = stream.read(&mut request);
                let body = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITestKey user@host\n\nssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQCyTestKey user@host\n";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let base_url = format!("http://{addr}");
        let keys = fetch_github_public_keys_from_base("alice", &base_url)
            .expect("github key fetch should parse response");

        assert_eq!(keys.len(), 2);
        assert!(keys[0].starts_with("ssh-ed25519"));
        assert!(keys[1].starts_with("ssh-rsa"));
    }
}
