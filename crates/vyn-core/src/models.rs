use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Persisted vault configuration, stored in `.vyn/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub vault_id: String,
    pub project_name: Option<String>,
    pub storage_provider: String,
    pub relay_url: Option<String>,
}

/// Persisted identity, stored in `.vyn/identity.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub github_username: String,
    pub ssh_private_key: String,
    pub ssh_public_key: String,
}

/// A single entry in the push/pull history log, stored under `.vyn/history/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp_unix: u64,
    /// `"push"` or `"pull"`.
    pub source: String,
    pub manifest_version: u64,
    pub file_count: usize,
}

/// Global per-user config, stored at `~/.config/vyn/global.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// How vyn was installed: `"binary"`, `"cargo"`, `"docker"`, or absent when unknown.
    pub install_method: Option<String>,
    /// Unix timestamp (seconds) of the last GitHub releases API check.
    pub last_version_check_unix: Option<u64>,
    /// Latest version string seen at the last check (e.g. `"0.1.3"`).
    pub latest_known_version: Option<String>,
}

/// Returns the path to `~/.config/vyn/global.toml` (XDG-compliant).
pub fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("vyn").join("global.toml"))
}

/// Loads `GlobalConfig` from disk, returning a default value if missing or malformed.
pub fn load_global_config() -> GlobalConfig {
    let path = match global_config_path() {
        Some(p) => p,
        None => return GlobalConfig::default(),
    };
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return GlobalConfig::default(),
    };
    toml::from_str(&text).unwrap_or_default()
}

/// Persists `GlobalConfig` to disk, creating the directory if needed.
pub fn save_global_config(cfg: &GlobalConfig) -> Result<()> {
    let path = global_config_path().context("cannot determine config directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("cannot create config directory")?;
    }
    let text = toml::to_string(cfg).context("cannot serialize global config")?;
    fs::write(&path, text).context("cannot write global config")?;
    Ok(())
}
