use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use vyn_core::relay_storage::RelayStorageProvider;

use crate::output;

#[derive(Debug, Deserialize)]
struct VaultConfig {
    #[allow(dead_code)]
    vault_id: String,
    relay_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdentityConfig {
    github_username: String,
    #[allow(dead_code)]
    ssh_private_key: String,
    ssh_public_key: String,
}

pub fn run_status() -> Result<()> {
    output::print_banner("relay status");
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let vault_dir = root.join(".vyn");

    let relay_url = load_relay_url(&root)?;
    let identity = load_identity(&vault_dir)?;

    output::print_info("relay", &relay_url);
    output::print_info(
        "identity",
        &format!(
            "@{} ({})",
            identity.github_username,
            ssh_key_fingerprint(&identity.ssh_public_key)
                .unwrap_or_else(|| "<unreadable>".to_string())
        ),
    );

    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(async {
        let provider = RelayStorageProvider::new(relay_url.clone());
        let spinner = output::new_spinner("checking connectivity and auth…");
        match provider.authenticate_with_identity(&vault_dir).await {
            Ok(_) => {
                output::finish_progress(&spinner, "authenticated");
                output::print_info("auth", "OK");
            }
            Err(e) => {
                output::fail_progress(&spinner, &format!("auth failed: {e}"));
                output::print_info("auth", &format!("FAILED -- {e}"));
            }
        }
        Ok::<(), anyhow::Error>(())
    })?;

    println!();
    Ok(())
}

fn load_relay_url(root: &Path) -> Result<String> {
    // Try .vyn/config.toml first, then fall back to vyn.toml.
    let config_path = root.join(".vyn").join("config.toml");
    if config_path.exists()
        && let Ok(text) = fs::read_to_string(&config_path)
        && let Ok(cfg) = toml::from_str::<VaultConfig>(&text)
        && let Some(url) = cfg.relay_url
    {
        return Ok(url);
    }
    let vyn_toml = root.join("vyn.toml");
    if vyn_toml.exists()
        && let Ok(text) = fs::read_to_string(&vyn_toml)
    {
        #[derive(Deserialize)]
        struct PublicConfig {
            relay_url: Option<String>,
        }
        if let Ok(pc) = toml::from_str::<PublicConfig>(&text)
            && let Some(url) = pc.relay_url
        {
            return Ok(url);
        }
    }
    anyhow::bail!("no relay_url configured; run `vyn config` to set it")
}

fn load_identity(vault_dir: &Path) -> Result<IdentityConfig> {
    let path = vault_dir.join("identity.toml");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing identity.toml at {}", path.display()))?;
    toml::from_str(&text).context("invalid identity.toml format")
}

fn ssh_key_fingerprint(pubkey_path: &str) -> Option<String> {
    let content = fs::read_to_string(pubkey_path).ok()?;
    let trimmed = content.trim();
    // Return the last field (comment) if present, otherwise the key type.
    let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
    if parts.len() >= 3 {
        Some(format!(
            "{}:{}",
            parts[0],
            parts[2].split_whitespace().next().unwrap_or("")
        ))
    } else if !parts.is_empty() {
        Some(parts[0].to_string())
    } else {
        None
    }
}

pub fn run_ls(_vault: Option<String>) -> Result<()> {
    output::print_banner("relay ls");
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let vault_dir = root.join(".vyn");
    let relay_url = load_relay_url(&root)?;

    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(async {
        let provider = RelayStorageProvider::new(relay_url);
        let spinner = output::new_spinner("authenticating…");
        provider
            .authenticate_with_identity(&vault_dir)
            .await
            .context("relay authentication failed (run `vyn auth` first)")?;
        output::finish_progress(&spinner, "authenticated");

        let spinner2 = output::new_spinner("listing vaults…");
        let vaults = provider
            .list_vaults()
            .await
            .context("failed to list vaults")?;
        output::finish_progress(&spinner2, &format!("{} vault(s)", vaults.len()));

        if vaults.is_empty() {
            println!("No vaults found.");
            return Ok::<(), anyhow::Error>(());
        }

        let spinner3 = output::new_spinner("listing blobs…");
        let blobs = provider
            .list_blobs()
            .await
            .context("failed to list blobs")?;
        output::finish_progress(&spinner3, &format!("{} blob(s) total", blobs.len()));

        println!();
        println!("Vaults:");
        for vault_id in &vaults {
            println!("  {vault_id}");
        }

        if !blobs.is_empty() {
            println!();
            println!("Blobs:");
            for (sha256, size) in &blobs {
                println!("  {}  ({} B)", sha256, size);
            }
        }

        println!();
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
