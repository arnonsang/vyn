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
