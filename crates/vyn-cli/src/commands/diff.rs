use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use secrecy::ExposeSecret;
use serde::Deserialize;
use vyn_core::blob::decrypt_blob_by_hash;
use vyn_core::diff::{is_binary, unified_diff};
use vyn_core::ignore::load_ignore_matcher;
use vyn_core::keychain::load_project_key;
use vyn_core::manifest::{FileEntry, Manifest, capture_manifest};

use crate::output;

#[derive(Debug, Deserialize)]
struct VaultConfig {
    vault_id: String,
}

pub fn run(file: Option<String>) -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let (manifest, vault_id) = load_vault_state(&root)?;

    let matcher = load_ignore_matcher(&root).context("failed to load ignore matcher")?;
    let current =
        capture_manifest(&root, &matcher).context("failed to capture current manifest")?;

    let old_by_path = map_by_path(&manifest.files);
    let new_by_path = map_by_path(&current.files);
    let changed = collect_changed_paths(&old_by_path, &new_by_path, file.as_deref())?;

    if changed.is_empty() {
        output::print_status_clean();
        return Ok(());
    }

    let needs_key = changed.iter().any(|p| old_by_path.contains_key(p));
    let key = if needs_key {
        Some(load_project_key(&vault_id).context("failed to load project key from keychain")?)
    } else {
        None
    };
    let blobs_dir = root.join(".vyn").join("blobs");

    for path in changed {
        output::print_diff_header(&path);

        let old_data = match old_by_path.get(&path) {
            Some(old_entry) => Some(
                decrypt_blob_by_hash(
                    &blobs_dir,
                    &old_entry.sha256,
                    key.as_ref().expect("key required for baseline diff"),
                )
                .with_context(|| format!("failed to decrypt baseline blob for {path}"))?,
            ),
            None => None,
        };

        let new_data = if root.join(&path).exists() {
            Some(
                fs::read(root.join(&path))
                    .with_context(|| format!("failed to read local file for diff: {path}"))?,
            )
        } else {
            None
        };

        let old_bytes = old_data
            .as_ref()
            .map(|s| s.expose_secret())
            .unwrap_or_default();
        let new_bytes = new_data.as_deref().unwrap_or_default();

        if is_binary(old_bytes) || is_binary(new_bytes) {
            output::print_binary_modified(&path, old_bytes.len(), new_bytes.len());
            continue;
        }

        let old_text = String::from_utf8_lossy(old_bytes);
        let new_text = String::from_utf8_lossy(new_bytes);
        let rendered = unified_diff(
            &old_text,
            &new_text,
            &format!("a/{path}"),
            &format!("b/{path}"),
        );
        output::print_diff_text(&rendered);
    }

    Ok(())
}

fn map_by_path(entries: &[FileEntry]) -> BTreeMap<String, FileEntry> {
    entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect()
}

fn collect_changed_paths(
    old_by_path: &BTreeMap<String, FileEntry>,
    new_by_path: &BTreeMap<String, FileEntry>,
    only_file: Option<&str>,
) -> Result<Vec<String>> {
    if let Some(target) = only_file {
        let target = target.replace('\\', "/");
        let old = old_by_path.get(&target).map(|e| &e.sha256);
        let new = new_by_path.get(&target).map(|e| &e.sha256);

        if old.is_none() && new.is_none() {
            anyhow::bail!("file not found in local tree or baseline manifest: {target}");
        }
        if old == new {
            return Ok(Vec::new());
        }

        return Ok(vec![target]);
    }

    let mut paths: BTreeSet<String> = BTreeSet::new();
    paths.extend(old_by_path.keys().cloned());
    paths.extend(new_by_path.keys().cloned());

    Ok(paths
        .into_iter()
        .filter(|path| {
            old_by_path.get(path).map(|e| &e.sha256) != new_by_path.get(path).map(|e| &e.sha256)
        })
        .collect())
}

fn load_vault_state(root: &Path) -> Result<(Manifest, String)> {
    let vault_dir = root.join(".vyn");
    let config_path = vault_dir.join("config.toml");
    let manifest_path = vault_dir.join("manifest.json");

    let config_text = fs::read_to_string(&config_path)
        .with_context(|| format!("missing or unreadable file: {}", config_path.display()))?;
    let config: VaultConfig =
        toml::from_str(&config_text).context("invalid .vyn/config.toml format")?;

    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("missing or unreadable file: {}", manifest_path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&manifest_text).context("invalid .vyn/manifest.json format")?;

    Ok((manifest, config.vault_id))
}
