use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use secrecy::ExposeSecret;
use serde::Deserialize;
use vyn_core::blob::{blob_path as make_blob_path, decrypt_blob_bytes};
use vyn_core::keychain::load_project_key;
use vyn_core::relay_storage::RelayStorageProvider;
use vyn_core::storage::{InMemoryStorageProvider, StorageProvider, decrypt_manifest};

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
    async fn get_manifest(&self, project_id: &str) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            Provider::Memory(p) => p.get_manifest(project_id).await,
            Provider::Relay(p) => p.get_manifest(project_id).await,
        }
        .context("get_manifest failed")
    }

    async fn download_blob(&self, hash: &str) -> anyhow::Result<Option<Vec<u8>>> {
        match self {
            Provider::Memory(p) => p.download_blob(hash).await,
            Provider::Relay(p) => p.download_blob(hash).await,
        }
        .context("download_blob failed")
    }
}

pub fn run() -> Result<()> {
    output::print_banner("pull");
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let config = load_config(&root)?;
    let key =
        load_project_key(&config.vault_id).context("failed to load project key from keychain")?;
    let vault_id = config.vault_id.clone();

    let vault_dir = root.join(".vyn");
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    let manifest = runtime.block_on(async {
        let provider = provider_for_config(&config, &vault_dir).await?;

        let spinner = output::new_spinner("fetching manifest…");
        let payload = provider
            .get_manifest(&vault_id)
            .await
            .context("failed to fetch remote manifest")?
            .with_context(|| format!("no remote manifest found for vault {}", vault_id))?;
        let manifest =
            decrypt_manifest(&payload, &key).context("failed to decrypt remote manifest")?;
        output::finish_progress(
            &spinner,
            &format!("{} files in manifest", manifest.files.len()),
        );

        let blobs_dir = root.join(".vyn").join("blobs");
        fs::create_dir_all(&blobs_dir).context("failed to create blobs directory")?;

        let pb = output::new_progress_bar(manifest.files.len() as u64, "downloading blobs");
        for entry in &manifest.files {
            // Download ciphertext from relay
            let encrypted_bytes = provider
                .download_blob(&entry.sha256)
                .await
                .with_context(|| format!("failed to download blob {}", entry.sha256))?
                .with_context(|| format!("missing blob {}", entry.sha256))?;

            // Cache encrypted blob locally for offline diff
            let blob_file = make_blob_path(&blobs_dir, &entry.sha256);
            fs::write(&blob_file, &encrypted_bytes)
                .with_context(|| format!("failed to cache blob {}", entry.sha256))?;

            // Decrypt and write to disk
            let plaintext = decrypt_blob_bytes(&blob_file, &key)
                .with_context(|| format!("failed to decrypt blob {}", entry.sha256))?;
            let dest = root.join(&entry.path);
            // Reject path traversal by normalizing without requiring the path to exist
            let normalized = dest
                .components()
                .fold(std::path::PathBuf::new(), |mut acc, c| {
                    acc.push(c);
                    acc
                });
            if !normalized.starts_with(&root) {
                anyhow::bail!("manifest contains unsafe path: {}", entry.path);
            }
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            fs::write(&dest, plaintext.expose_secret())
                .with_context(|| format!("failed to write file {}", dest.display()))?;
            pb.inc(1);
        }
        output::finish_progress(&pb, "blobs written to disk");

        Ok::<_, anyhow::Error>(manifest)
    })?;

    let manifest_path = root.join(".vyn").join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?,
    )
    .with_context(|| format!("failed to write {}", manifest_path.display()))?;

    write_history_entry(&root, "pull", manifest.version, manifest.files.len())
        .context("failed to record pull history")?;

    output::print_success(&format!("vault {} pulled", config.vault_id));
    output::print_info("files", &format!("{} synced", manifest.files.len()));
    println!();
    Ok(())
}

fn load_config(root: &Path) -> Result<VaultConfig> {
    let path = root.join(".vyn").join("config.toml");
    if path.exists() {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("missing or unreadable file: {}", path.display()))?;
        return toml::from_str(&text).context("invalid .vyn/config.toml format");
    }
    // Fallback: read vault_id (and optionally relay_url) from committed vyn.toml.
    let vyn_toml = root.join("vyn.toml");
    let text = fs::read_to_string(&vyn_toml)
        .context("no .vyn/config.toml found; run `vyn clone` or `vyn link` first")?;
    #[derive(serde::Deserialize)]
    struct PublicConfig {
        vault_id: String,
        relay_url: Option<String>,
    }
    let pc: PublicConfig = toml::from_str(&text).context("invalid vyn.toml format")?;
    let storage_provider = if pc.relay_url.is_some() {
        "relay".to_string()
    } else {
        "unconfigured".to_string()
    };
    Ok(VaultConfig {
        vault_id: pc.vault_id,
        storage_provider,
        relay_url: pc.relay_url,
    })
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
            let spinner = crate::output::new_spinner("authenticating with relay...");
            if let Err(e) = provider.authenticate_with_identity(vault_dir).await {
                let msg = e.to_string();
                if msg.contains("invalid or expired token") || msg.contains("session expired") {
                    RelayStorageProvider::clear_cached_token(vault_dir);
                    crate::output::fail_progress(
                        &spinner,
                        "relay session expired - run `vyn auth` to re-authenticate",
                    );
                    return Err(anyhow::anyhow!(
                        "relay session expired - run `vyn auth` to re-authenticate"
                    ));
                }
                crate::output::fail_progress(&spinner, "relay authentication failed");
                return Err(anyhow::anyhow!(e)
                    .context("relay authentication failed (run `vyn auth` first)"));
            }
            crate::output::finish_progress(&spinner, "authenticated");
            Ok(Provider::Relay(provider))
        }
        "unconfigured" => anyhow::bail!(
            "storage provider is unconfigured; set `storage_provider = \"memory\"` or `\"relay\"` in .vyn/config.toml"
        ),
        other => anyhow::bail!("unsupported storage provider `{other}`"),
    }
}
