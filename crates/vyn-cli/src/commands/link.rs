use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use vyn_core::keychain::store_project_key;
use vyn_core::wrapping::unwrap_project_key_with_ssh_identity_file;

use crate::output;

#[derive(Debug, Deserialize)]
struct IdentityConfig {
    github_username: String,
    ssh_private_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct VaultConfig {
    vault_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    storage_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    relay_url: Option<String>,
}

pub fn run(vault_id: String) -> Result<()> {
    output::print_banner("link");
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let identity = load_identity(&root)?;

    let invites_dir = root.join(".vyn").join("invites");
    let prefix = format!("{}__{}__", vault_id, identity.github_username);
    let invite_path = fs::read_dir(&invites_dir)
        .with_context(|| format!("failed to read invite directory: {}", invites_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|v| v.to_str())
                .map(|name| name.starts_with(&prefix) && name.ends_with(".age"))
                .unwrap_or(false)
        })
        .with_context(|| {
            format!(
                "no invite found for vault {} and user {} in {}",
                vault_id,
                identity.github_username,
                invites_dir.display()
            )
        })?;

    let spinner = output::new_spinner("decrypting invite…");
    let payload = fs::read(&invite_path)
        .with_context(|| format!("failed to read invite file: {}", invite_path.display()))?;
    let key =
        unwrap_project_key_with_ssh_identity_file(&payload, Path::new(&identity.ssh_private_key))
            .context("failed to decrypt invite with SSH private key")?;
    output::finish_progress(&spinner, "invite decrypted");

    store_project_key(&vault_id, &key).context("failed to store project key in keychain")?;

    // Update config.toml to point at the linked vault id so push/pull work immediately
    let config_path = root.join(".vyn").join("config.toml");
    if let Ok(text) = fs::read_to_string(&config_path) {
        if let Ok(mut cfg) = toml::from_str::<VaultConfig>(&text) {
            cfg.vault_id = vault_id.clone();
            if let Ok(serialized) = toml::to_string_pretty(&cfg) {
                let _ = fs::write(&config_path, serialized);
            }
        }
    }

    output::print_success(&format!("vault {vault_id} linked"));
    output::print_info("identity", &format!("@{}", identity.github_username));
    output::print_info("key stored", "OS keychain");
    println!();
    Ok(())
}

fn load_identity(root: &Path) -> Result<IdentityConfig> {
    let path = root.join(".vyn").join("identity.toml");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing or unreadable file: {}", path.display()))?;
    toml::from_str(&text).context("invalid .vyn/identity.toml format")
}
