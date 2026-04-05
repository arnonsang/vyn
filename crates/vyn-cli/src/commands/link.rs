use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use vyn_core::keychain::store_project_key;
use vyn_core::relay_storage::RelayStorageProvider;
use vyn_core::storage::StorageProvider;
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
    let vault_dir = root.join(".vyn");

    let relay_url = load_relay_url(&root)?;

    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(async {
        let provider = RelayStorageProvider::new(relay_url);
        provider
            .authenticate_with_identity(&vault_dir)
            .await
            .context("relay authentication failed (run `vyn auth` first)")?;

        let spinner = output::new_spinner("fetching invite from relay…");
        let invites = provider
            .get_invites(&identity.github_username, &vault_id)
            .await
            .context("failed to fetch invites from relay")?;

        if invites.is_empty() {
            output::fail_progress(&spinner, "no invite found");
            anyhow::bail!(
                "no invite found for @{} in vault {vault_id}\nAsk a teammate to run: vyn share @{}",
                identity.github_username,
                identity.github_username
            );
        }
        output::finish_progress(&spinner, &format!("{} invite(s) found", invites.len()));

        let spinner2 = output::new_spinner("decrypting invite…");
        let key = invites
            .iter()
            .find_map(|payload| {
                unwrap_project_key_with_ssh_identity_file(
                    payload,
                    Path::new(&identity.ssh_private_key),
                )
                .ok()
            })
            .with_context(|| {
                format!(
                    "failed to decrypt any invite with SSH key at {}",
                    identity.ssh_private_key
                )
            })?;
        output::finish_progress(&spinner2, "invite decrypted");

        store_project_key(&vault_id, &key)
            .context("failed to store project key in keychain")?;

        let config_path = vault_dir.join("config.toml");
        if let Ok(text) = fs::read_to_string(&config_path)
            && let Ok(mut cfg) = toml::from_str::<VaultConfig>(&text)
        {
            cfg.vault_id = vault_id.clone();
            if let Ok(serialized) = toml::to_string_pretty(&cfg) {
                let _ = fs::write(&config_path, serialized);
            }
        }

        output::print_success(&format!("vault {vault_id} linked"));
        output::print_info("identity", &format!("@{}", identity.github_username));
        output::print_info("key stored", "OS keychain");
        println!();

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn load_identity(root: &Path) -> Result<IdentityConfig> {
    let path = root.join(".vyn").join("identity.toml");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing or unreadable file: {}", path.display()))?;
    toml::from_str(&text).context("invalid .vyn/identity.toml format")
}

fn load_relay_url(root: &Path) -> Result<String> {
    let config_path = root.join(".vyn").join("config.toml");
    let text = fs::read_to_string(&config_path)
        .with_context(|| format!("missing or unreadable file: {}", config_path.display()))?;
    let cfg: VaultConfig = toml::from_str(&text).context("invalid .vyn/config.toml format")?;
    cfg.relay_url
        .context("missing `relay_url` in .vyn/config.toml — run `vyn config` to set it")
}
