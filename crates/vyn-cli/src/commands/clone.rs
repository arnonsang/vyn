use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use vyn_core::keychain::store_project_key;
use vyn_core::relay_storage::RelayStorageProvider;
use vyn_core::storage::StorageProvider;
use vyn_core::wrapping::unwrap_invite_with_ssh_identity_file;

use crate::output;

#[derive(Debug, Deserialize, Serialize)]
struct IdentityConfig {
    github_username: String,
    ssh_private_key: String,
    ssh_public_key: String,
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

/// `vyn clone <relay_url> <vault_id>`
///
/// Full onboarding flow:
/// 1. Verify identity.toml exists.
/// 2. Create .vyn/config.toml with relay_url + vault_id.
/// 3. Fetch invite from relay, decrypt, store key in keychain.
/// 4. Write vyn.toml (committed public file).
/// 5. Run pull to download all blobs.
pub fn run(relay_url: String, vault_id: String) -> Result<()> {
    output::print_banner("clone");
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let vault_dir = root.join(".vyn");

    // Step 1: identity must exist.
    let identity = load_identity(&vault_dir)
        .context("no identity found -- run `vyn auth` first to register your GitHub identity")?;

    // Step 2: bootstrap .vyn/config.toml.
    fs::create_dir_all(&vault_dir).context("failed to create .vyn directory")?;

    // Ensure identity.toml is present in the local .vyn/ dir for relay auth.
    let identity_path = vault_dir.join("identity.toml");
    if !identity_path.exists() {
        let identity_toml =
            toml::to_string_pretty(&identity).context("failed to serialize identity")?;
        fs::write(&identity_path, identity_toml).context("failed to write .vyn/identity.toml")?;
    }

    let config_path = vault_dir.join("config.toml");
    let initial_cfg = VaultConfig {
        vault_id: vault_id.clone(),
        project_name: None,
        storage_provider: "relay".to_string(),
        relay_url: Some(relay_url.clone()),
    };
    fs::write(
        &config_path,
        toml::to_string_pretty(&initial_cfg).context("failed to serialize config")?,
    )
    .context("failed to write .vyn/config.toml")?;

    // Step 3: fetch and decrypt invite.
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    let invite = runtime.block_on(async {
        let provider = RelayStorageProvider::new(relay_url.clone());
        let spinner_auth = output::new_spinner("authenticating with relay…");
        provider
            .authenticate_with_identity(&vault_dir)
            .await
            .context("relay authentication failed (run `vyn auth` first)")?;
        output::finish_progress(&spinner_auth, "authenticated");

        let spinner = output::new_spinner("fetching invite from relay…");
        let invites = provider
            .get_invites(&identity.github_username, &vault_id)
            .await
            .context("failed to fetch invites from relay")?;

        if invites.is_empty() {
            output::fail_progress(&spinner, "no invite found");
            anyhow::bail!(
                "no invite found for @{u} in vault {vault_id}\n\
                 Ask a teammate to run: vyn share @{u}",
                u = identity.github_username
            );
        }
        output::finish_progress(&spinner, &format!("{} invite(s) found", invites.len()));

        let spinner2 = output::new_spinner("decrypting invite…");
        let invite = invites
            .iter()
            .find_map(|payload| {
                unwrap_invite_with_ssh_identity_file(payload, Path::new(&identity.ssh_private_key))
                    .ok()
            })
            .with_context(|| {
                format!(
                    "failed to decrypt any invite with SSH key at {}",
                    identity.ssh_private_key
                )
            })?;
        output::finish_progress(&spinner2, "invite decrypted");

        Ok::<_, anyhow::Error>(invite)
    })?;

    // Update vault_id from invite if embedded (handles server-assigned IDs).
    let resolved_vault_id = if !invite.vault_id.is_empty() {
        invite.vault_id.clone()
    } else {
        vault_id.clone()
    };

    // Store vault key in keychain.
    store_project_key(&resolved_vault_id, &invite.key)
        .context("failed to store project key in keychain")?;

    // Update config with any relay_url embedded in the invite.
    let final_relay_url = invite.relay_url.as_deref().unwrap_or(&relay_url);
    let final_cfg = VaultConfig {
        vault_id: resolved_vault_id.clone(),
        project_name: None,
        storage_provider: "relay".to_string(),
        relay_url: Some(final_relay_url.to_string()),
    };
    if let Ok(serialized) = toml::to_string_pretty(&final_cfg) {
        let _ = fs::write(&config_path, serialized);
    }

    // Step 4: write vyn.toml (public committed file).
    let vyn_toml_path = root.join("vyn.toml");
    if !vyn_toml_path.exists() {
        let vyn_toml = format!(
            "# Public vault config -- commit this file.\n\
             vault_id = \"{resolved_vault_id}\"\n\
             relay_url = \"{final_relay_url}\"\n"
        );
        let _ = fs::write(&vyn_toml_path, vyn_toml);
    }

    output::print_success(&format!("vault {resolved_vault_id} linked"));
    output::print_info("identity", &format!("@{}", identity.github_username));
    output::print_info("relay", final_relay_url);
    output::print_info("key stored", "OS keychain");
    println!();

    // Step 5: pull files.
    output::print_info("next", "pulling files…");
    crate::commands::pull::run()
}

fn load_identity(vault_dir: &Path) -> Result<IdentityConfig> {
    // Check local .vyn/identity.toml first, then fall back to ~/.vyn/identity.toml.
    let local_path = vault_dir.join("identity.toml");
    if local_path.exists() {
        let text = fs::read_to_string(&local_path)
            .with_context(|| format!("failed to read {}", local_path.display()))?;
        return toml::from_str(&text).context("invalid identity.toml format");
    }
    if let Some(home) = dirs::home_dir() {
        let global_path = home.join(".vyn").join("identity.toml");
        if global_path.exists() {
            let text = fs::read_to_string(&global_path)
                .with_context(|| format!("failed to read {}", global_path.display()))?;
            return toml::from_str(&text).context("invalid identity.toml format");
        }
    }
    anyhow::bail!("missing identity.toml at {}", local_path.display())
}
