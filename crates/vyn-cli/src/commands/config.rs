use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use dialoguer::{Input, Select};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ConfigOptions {
    pub provider: Option<String>,
    pub relay_url: Option<String>,
    pub non_interactive: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct VaultConfig {
    vault_id: String,
    project_name: Option<String>,
    storage_provider: String,
    relay_url: Option<String>,
}

pub fn run(opts: ConfigOptions) -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let config_path = root.join(".vyn").join("config.toml");

    let mut config = load_config(&config_path)?;

    if opts.non_interactive || opts.provider.is_some() {
        apply_non_interactive(&mut config, opts)?;
    } else {
        run_wizard(&mut config)?;
    }

    fs::write(
        &config_path,
        toml::to_string_pretty(&config).context("failed to serialize config")?,
    )
    .with_context(|| format!("failed to write {}", config_path.display()))?;

    println!(
        "config updated: provider={} vault_id={}",
        config.storage_provider, config.vault_id
    );

    Ok(())
}

fn load_config(path: &Path) -> Result<VaultConfig> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing or unreadable file: {}", path.display()))?;
    toml::from_str(&text).context("invalid .vyn/config.toml format")
}

fn apply_non_interactive(config: &mut VaultConfig, opts: ConfigOptions) -> Result<()> {
    let ConfigOptions {
        provider,
        relay_url,
        non_interactive: _,
    } = opts;

    let provider = provider.unwrap_or_else(|| config.storage_provider.clone());
    validate_provider(&provider)?;

    config.storage_provider = provider.clone();

    if let Some(relay_url) = relay_url {
        config.relay_url = normalize_opt(relay_url);
    }

    if provider == "relay" {
        let relay_url = config
            .relay_url
            .clone()
            .with_context(|| "relay provider requires --relay-url")?;
        config.relay_url = normalize_opt(relay_url);
    } else if provider == "memory" {
        config.relay_url = None;
    }

    Ok(())
}

fn run_wizard(config: &mut VaultConfig) -> Result<()> {
    let providers = ["memory", "relay"];
    let current_idx = if config.storage_provider == "relay" {
        1
    } else {
        0
    };

    let idx = Select::new()
        .with_prompt("Select storage provider")
        .items(&providers)
        .default(current_idx)
        .interact()
        .context("failed to read storage provider selection")?;

    let provider = providers[idx].to_string();
    config.storage_provider = provider.clone();

    if config.storage_provider == "relay" {
        let relay_default = config.relay_url.clone().unwrap_or_default();
        let relay: String = Input::new()
            .with_prompt("Relay URL (required for relay provider)")
            .default(relay_default)
            .allow_empty(true)
            .interact_text()
            .context("failed to read relay URL")?;
        config.relay_url = normalize_opt(relay);

        if config.relay_url.is_none() {
            anyhow::bail!("relay provider requires relay_url");
        }
    } else {
        config.relay_url = None;
    }

    Ok(())
}

fn validate_provider(provider: &str) -> Result<()> {
    match provider {
        "memory" | "relay" => Ok(()),
        other => anyhow::bail!("unsupported provider '{other}'; expected memory or relay"),
    }
}

fn normalize_opt(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
