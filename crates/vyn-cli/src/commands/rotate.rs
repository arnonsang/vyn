use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use vyn_core::blob::{blob_path as make_blob_path, encrypt_file_to_blob};
use vyn_core::crypto::{SecretBytes, generate_project_key};
use vyn_core::ignore::load_ignore_matcher;
use vyn_core::keychain::store_project_key;
use vyn_core::manifest::capture_manifest;
use vyn_core::relay_storage::RelayStorageProvider;
use vyn_core::storage::{InMemoryStorageProvider, StorageProvider, encrypt_manifest};
use vyn_core::wrapping::wrap_project_key_for_ssh_recipient;

use crate::commands::history::write_history_entry;

#[derive(Debug, Deserialize)]
struct VaultConfig {
    vault_id: String,
    storage_provider: String,
    relay_url: Option<String>,
}

static MEMORY_PROVIDER: OnceLock<InMemoryStorageProvider> = OnceLock::new();

enum Provider {
    Memory(InMemoryStorageProvider),
    Relay(RelayStorageProvider),
}

impl Provider {
    async fn upload_blob(&self, hash: &str, data: Vec<u8>) -> anyhow::Result<()> {
        match self {
            Provider::Memory(p) => p.upload_blob(hash, data).await,
            Provider::Relay(p) => p.upload_blob(hash, data).await,
        }
        .context("upload_blob failed")
    }

    async fn put_manifest(&self, project_id: &str, payload: &[u8]) -> anyhow::Result<()> {
        match self {
            Provider::Memory(p) => p.put_manifest(project_id, payload).await,
            Provider::Relay(p) => p.put_manifest(project_id, payload).await,
        }
        .context("put_manifest failed")
    }
}

pub fn run() -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let config = load_config(&root)?;
    let matcher = load_ignore_matcher(&root).context("failed to load ignore matcher")?;
    let manifest = capture_manifest(&root, &matcher).context("failed to capture manifest")?;
    let new_key = generate_project_key().context("failed to generate replacement project key")?;

    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    let vault_dir = root.join(".vyn");
    let blobs_dir = vault_dir.join("blobs");
    fs::create_dir_all(&blobs_dir).context("failed to create blobs directory")?;

    runtime.block_on(async {
        let provider = provider_for_config(&config, &vault_dir).await?;

        for entry in &manifest.files {
            let abs_path = root.join(&entry.path);
            let blob_file = make_blob_path(&blobs_dir, &entry.sha256);
            // Always re-encrypt with new_key
            encrypt_file_to_blob(&abs_path, &blobs_dir, &new_key)
                .with_context(|| format!("failed to encrypt blob {}", entry.sha256))?;
            let bytes = fs::read(&blob_file)
                .with_context(|| format!("failed to read encrypted blob {}", entry.sha256))?;
            provider
                .upload_blob(&entry.sha256, bytes)
                .await
                .with_context(|| format!("failed to upload blob {}", entry.sha256))?;
        }

        let payload =
            encrypt_manifest(&manifest, &new_key).context("failed to encrypt manifest")?;
        provider
            .put_manifest(&config.vault_id, &payload)
            .await
            .context("failed to upload rotated manifest")?;

        Ok::<(), anyhow::Error>(())
    })?;

    store_project_key(&config.vault_id, &new_key)
        .context("failed to store rotated project key in keychain")?;

    let manifest_path = root.join(".vyn").join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?,
    )
    .with_context(|| format!("failed to write {}", manifest_path.display()))?;

    let reshared = rotate_local_invites(&root, &config.vault_id, &new_key)?;

    write_history_entry(&root, "rotate", manifest.version, manifest.files.len())
        .context("failed to record rotate history")?;

    println!(
        "rotate completed for vault {} (re-shared with {} teammate(s))",
        config.vault_id, reshared
    );
    Ok(())
}

fn rotate_local_invites(root: &Path, vault_id: &str, new_key: &SecretBytes) -> Result<usize> {
    let invites_dir = root.join(".vyn").join("invites");
    if !invites_dir.exists() {
        return Ok(0);
    }

    let teammates = discover_teammates(&invites_dir, vault_id)?;
    if teammates.is_empty() {
        return Ok(0);
    }

    let mut reshared = 0usize;
    for teammate in teammates {
        let keys = fetch_github_public_keys(&teammate)
            .with_context(|| format!("failed to fetch GitHub keys for @{teammate}"))?;
        if keys.is_empty() {
            continue;
        }

        remove_existing_invites(&invites_dir, vault_id, &teammate)?;

        let mut created_for_user = 0usize;
        for (idx, key) in keys.iter().enumerate() {
            if let Ok(payload) = wrap_project_key_for_ssh_recipient(new_key, key) {
                let path = invites_dir.join(format!("{}__{}__{}.age", vault_id, teammate, idx));
                fs::write(&path, payload)
                    .with_context(|| format!("failed to write invite file: {}", path.display()))?;
                created_for_user += 1;
            }
        }

        if created_for_user > 0 {
            reshared += 1;
        }
    }

    Ok(reshared)
}

fn discover_teammates(invites_dir: &Path, vault_id: &str) -> Result<BTreeSet<String>> {
    let mut users = BTreeSet::new();
    let prefix = format!("{}__", vault_id);

    for entry in fs::read_dir(invites_dir)
        .with_context(|| format!("failed to read invite directory: {}", invites_dir.display()))?
    {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if !name.starts_with(&prefix) || !name.ends_with(".age") {
            continue;
        }

        if let Some(user) = parse_username_from_invite_name(name, vault_id)
            && !user.is_empty()
        {
            users.insert(user);
        }
    }

    Ok(users)
}

fn parse_username_from_invite_name(file_name: &str, vault_id: &str) -> Option<String> {
    let marker = format!("{}__", vault_id);
    let rest = file_name.strip_prefix(&marker)?;
    let username = rest.split("__").next()?;
    Some(username.to_string())
}

fn remove_existing_invites(invites_dir: &Path, vault_id: &str, username: &str) -> Result<()> {
    let prefix = format!("{}__{}__", vault_id, username);

    for entry in fs::read_dir(invites_dir)
        .with_context(|| format!("failed to read invite directory: {}", invites_dir.display()))?
    {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if name.starts_with(&prefix) && name.ends_with(".age") {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove old invite file: {}", path.display()))?;
        }
    }

    Ok(())
}

fn load_config(root: &Path) -> Result<VaultConfig> {
    let path = root.join(".vyn").join("config.toml");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing or unreadable file: {}", path.display()))?;
    toml::from_str(&text).context("invalid .vyn/config.toml format")
}

async fn provider_for_config(config: &VaultConfig, vault_dir: &Path) -> Result<Provider> {
    match config.storage_provider.as_str() {
        "memory" => Ok(Provider::Memory(
            MEMORY_PROVIDER
                .get_or_init(InMemoryStorageProvider::new)
                .clone(),
        )),
        "relay" => {
            let relay_url = config
                .relay_url
                .clone()
                .context("missing `relay_url` for storage_provider = \"relay\"")?;
            let provider = RelayStorageProvider::new(relay_url);
            provider
                .authenticate_with_identity(vault_dir)
                .await
                .context("relay authentication failed (run `vyn auth` first)")?;
            Ok(Provider::Relay(provider))
        }
        "unconfigured" => anyhow::bail!(
            "storage provider is unconfigured; set `storage_provider = \"memory\"` or `\"relay\"` in .vyn/config.toml"
        ),
        other => anyhow::bail!("unsupported storage provider `{other}`"),
    }
}

fn fetch_github_public_keys(username: &str) -> Result<Vec<String>> {
    let url = format!("https://github.com/{username}.keys");
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

    Ok(body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::parse_username_from_invite_name;

    #[test]
    fn parse_invite_name_username() {
        let username = parse_username_from_invite_name("vault-123__alice__0.age", "vault-123")
            .expect("username should parse");
        assert_eq!(username, "alice");
        assert!(parse_username_from_invite_name("invalid.age", "vault-123").is_none());
    }
}
