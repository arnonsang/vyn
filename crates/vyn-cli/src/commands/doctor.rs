use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use vyn_core::keychain::load_project_key;
use vyn_core::manifest::Manifest;

use crate::output;
use crate::version::{VersionStatus, check_for_update};

#[derive(Debug, Deserialize)]
struct VaultConfig {
    vault_id: String,
    storage_provider: String,
    relay_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdentityConfig {
    github_username: String,
    ssh_private_key: String,
    ssh_public_key: String,
}

#[derive(Debug)]
struct CheckResult {
    name: String,
    ok: bool,
    detail: String,
}

pub fn run() -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let results = run_checks(&root)?;

    output::print_banner("doctor");
    let mut failures = 0usize;
    for result in &results {
        output::print_check_row(&result.name, result.ok, &result.detail);
        if !result.ok {
            failures += 1;
        }
    }
    println!();

    if failures > 0 {
        output::print_error(&format!("{failures} check(s) failed"));
        println!();
        anyhow::bail!("doctor found {failures} failing checks");
    }

    output::print_success("all checks passed");
    Ok(())
}

fn run_checks(root: &Path) -> Result<Vec<CheckResult>> {
    let mut out = Vec::new();

    // Version check is always first -- most immediately actionable info.
    let current = env!("CARGO_PKG_VERSION");
    match check_for_update(true) {
        VersionStatus::UpdateAvailable(latest) => out.push(fail(
            "cli_version",
            &format!("vyn v{current} installed, v{latest} available -- run 'vyn update'"),
        )),
        VersionStatus::UpToDate => out.push(ok("cli_version", &format!("vyn v{current} (latest)"))),
        VersionStatus::CheckFailed => out.push(ok(
            "cli_version",
            &format!("vyn v{current} (could not check for updates)"),
        )),
    }

    let vault_dir = root.join(".vyn");
    if vault_dir.exists() && vault_dir.is_dir() {
        out.push(ok("vault_directory", ".vyn exists"));
    } else {
        out.push(fail("vault_directory", ".vyn directory is missing"));
        return Ok(out);
    }

    let config_path = vault_dir.join("config.toml");
    let config_text = match fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(err) => {
            out.push(fail(
                "config_file",
                &format!("cannot read {}: {err}", config_path.display()),
            ));
            return Ok(out);
        }
    };

    let config: VaultConfig = match toml::from_str(&config_text) {
        Ok(c) => {
            out.push(ok("config_file", "config parsed"));
            c
        }
        Err(err) => {
            out.push(fail("config_file", &format!("invalid config.toml: {err}")));
            return Ok(out);
        }
    };

    match load_project_key(&config.vault_id) {
        Ok(_) => out.push(ok("keychain", "project key loaded")),
        Err(err) => out.push(fail("keychain", &format!("cannot load project key: {err}"))),
    }

    let manifest_path = vault_dir.join("manifest.json");
    match fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Manifest>(&text).ok())
    {
        Some(manifest) => out.push(ok(
            "manifest",
            &format!("manifest readable ({} file entries)", manifest.files.len()),
        )),
        None => out.push(fail("manifest", "missing or invalid .vyn/manifest.json")),
    }

    let identity_path = vault_dir.join("identity.toml");
    match fs::read_to_string(&identity_path)
        .ok()
        .and_then(|text| toml::from_str::<IdentityConfig>(&text).ok())
    {
        Some(identity) => {
            let priv_exists = Path::new(&identity.ssh_private_key).exists();
            let pub_exists = Path::new(&identity.ssh_public_key).exists();
            if priv_exists && pub_exists {
                out.push(ok(
                    "identity",
                    &format!("identity loaded for @{}", identity.github_username),
                ));
            } else {
                out.push(fail(
                    "identity",
                    "identity exists but ssh key files are missing",
                ));
            }
        }
        None => out.push(fail("identity", "missing or invalid .vyn/identity.toml")),
    }

    let relay_status = if let Some(url) = &config.relay_url {
        if url.starts_with("http://") || url.starts_with("https://") {
            ok("relay_config", &format!("relay URL configured: {url}"))
        } else {
            fail(
                "relay_config",
                "relay_url must start with http:// or https://",
            )
        }
    } else {
        fail("relay_config", "relay_url is not configured")
    };
    out.push(relay_status);

    match config.storage_provider.as_str() {
        "memory" => out.push(ok("storage", "memory storage configured")),
        "relay" => {
            if config.relay_url.is_none() {
                out.push(fail("storage", "relay configured but relay_url missing"));
            } else {
                out.push(ok("storage", "relay storage configured"));
            }
        }
        "unconfigured" => out.push(fail(
            "storage",
            "storage_provider is unconfigured; use memory or relay",
        )),
        other => out.push(fail(
            "storage",
            &format!("unsupported storage provider: {other}"),
        )),
    }

    Ok(out)
}

fn ok(name: &str, detail: &str) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        ok: true,
        detail: detail.to_string(),
    }
}

fn fail(name: &str, detail: &str) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        ok: false,
        detail: detail.to_string(),
    }
}
