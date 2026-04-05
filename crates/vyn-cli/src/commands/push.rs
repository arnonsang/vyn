use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::Deserialize;
use vyn_core::blob::{blob_path as make_blob_path, encrypt_file_to_blob};
use vyn_core::ignore::load_ignore_matcher;
use vyn_core::keychain::load_project_key;
use vyn_core::manifest::capture_manifest;
use vyn_core::relay_storage::RelayStorageProvider;
use vyn_core::storage::{InMemoryStorageProvider, StorageProvider, encrypt_manifest};

use crate::commands::history::write_history_entry;
use crate::output;

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
    output::print_banner("push");
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let config = load_config(&root)?;

    let spinner = output::new_spinner("scanning files…");
    let matcher = load_ignore_matcher(&root).context("failed to load ignore matcher")?;
    let manifest = capture_manifest(&root, &matcher).context("failed to capture manifest")?;
    output::finish_progress(&spinner, &format!("{} files to push", manifest.files.len()));

    let key =
        load_project_key(&config.vault_id).context("failed to load project key from keychain")?;
    let vault_id = config.vault_id.clone();
    let blobs_dir = root.join(".vyn").join("blobs");
    fs::create_dir_all(&blobs_dir).context("failed to create blobs directory")?;

    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    let vault_dir = root.join(".vyn");
    runtime.block_on(async {
        let provider = provider_for_config(&config, &vault_dir).await?;

        let total = manifest.files.len() as u64;
        let pb = output::new_progress_bar(total, "uploading blobs");
        for entry in &manifest.files {
            let abs_path = root.join(&entry.path);
            // Encrypt and cache locally: .vyn/blobs/<sha256>.enc
            let blob_file = make_blob_path(&blobs_dir, &entry.sha256);
            if !blob_file.exists() {
                encrypt_file_to_blob(&abs_path, &blobs_dir, &key)
                    .with_context(|| format!("failed to encrypt blob {}", entry.sha256))?;
            }
            // Upload ciphertext to relay
            let encrypted_bytes = fs::read(&blob_file)
                .with_context(|| format!("failed to read encrypted blob {}", entry.sha256))?;
            provider
                .upload_blob(&entry.sha256, encrypted_bytes)
                .await
                .with_context(|| format!("failed to upload blob {}", entry.sha256))?;
            pb.inc(1);
        }
        output::finish_progress(&pb, "blobs uploaded");

        let spinner2 = output::new_spinner("encrypting and uploading manifest…");
        let payload = encrypt_manifest(&manifest, &key).context("failed to encrypt manifest")?;
        provider
            .put_manifest(&vault_id, &payload)
            .await
            .context("failed to upload manifest")?;
        output::finish_progress(&spinner2, "manifest uploaded");

        let manifest_path = root.join(".vyn").join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?,
        )
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;

        write_history_entry(&root, "push", manifest.version, manifest.files.len())
            .context("failed to record push history")?;

        Ok::<(), anyhow::Error>(())
    })?;

    output::print_success(&format!("push complete: vault {}", config.vault_id));
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
