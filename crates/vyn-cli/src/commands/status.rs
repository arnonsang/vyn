use std::collections::BTreeMap;
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

pub fn run(verbose: bool) -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let (manifest, vault_id) = load_vault_state(&root)?;

    let matcher = load_ignore_matcher(&root).context("failed to load ignore matcher")?;
    let spinner = output::new_spinner("scanning files...");
    let current =
        capture_manifest(&root, &matcher).context("failed to capture current manifest")?;
    output::finish_progress(&spinner, &format!("{} files scanned", current.files.len()));

    let old_by_path = map_by_path(&manifest.files);
    let new_by_path = map_by_path(&current.files);

    let mut changed = Vec::new();
    let mut added = Vec::new();
    let mut deleted = Vec::new();

    for (path, old_entry) in &old_by_path {
        match new_by_path.get(path) {
            Some(new_entry) if new_entry.sha256 != old_entry.sha256 => changed.push(path.clone()),
            Some(_) => {}
            None => deleted.push(path.clone()),
        }
    }

    for path in new_by_path.keys() {
        if !old_by_path.contains_key(path) {
            added.push(path.clone());
        }
    }

    if changed.is_empty() && added.is_empty() && deleted.is_empty() {
        output::print_status_clean();
        return Ok(());
    }

    for path in &added {
        output::print_status_added(path);
    }
    for path in &changed {
        output::print_status_modified(path);
    }
    for path in &deleted {
        output::print_status_deleted(path);
    }

    if verbose {
        let needs_key = !changed.is_empty() || !deleted.is_empty();
        let key = if needs_key {
            Some(load_project_key(&vault_id).context("failed to load project key from keychain")?)
        } else {
            None
        };
        let blobs_dir = root.join(".vyn").join("blobs");

        for path in &added {
            let new_path = root.join(path);
            let new_data = fs::read(&new_path)
                .with_context(|| format!("failed to read local file for diff: {path}"))?;

            output::print_diff_header(path);
            if is_binary(&new_data) {
                output::print_binary_modified(path, 0, new_data.len());
                continue;
            }

            let new_text = String::from_utf8_lossy(&new_data);
            let rendered = unified_diff("", &new_text, &format!("a/{path}"), &format!("b/{path}"));
            output::print_diff_text(&rendered);
        }

        for path in &changed {
            let old_entry = old_by_path
                .get(path)
                .with_context(|| format!("missing old manifest entry for {path}"))?;

            let baseline = decrypt_blob_by_hash(
                &blobs_dir,
                &old_entry.sha256,
                key.as_ref().expect("key required for changed file"),
            )
            .with_context(|| format!("failed to decrypt baseline blob for {path}"))?;
            let new_data = fs::read(root.join(path))
                .with_context(|| format!("failed to read local file for diff: {path}"))?;

            output::print_diff_header(path);
            if is_binary(baseline.expose_secret()) || is_binary(&new_data) {
                output::print_binary_modified(path, baseline.expose_secret().len(), new_data.len());
                continue;
            }

            let old_text = String::from_utf8_lossy(baseline.expose_secret());
            let new_text = String::from_utf8_lossy(&new_data);
            let rendered = unified_diff(
                &old_text,
                &new_text,
                &format!("a/{path}"),
                &format!("b/{path}"),
            );
            output::print_diff_text(&rendered);
        }

        for path in &deleted {
            let old_entry = old_by_path
                .get(path)
                .with_context(|| format!("missing old manifest entry for {path}"))?;
            let baseline = decrypt_blob_by_hash(
                &blobs_dir,
                &old_entry.sha256,
                key.as_ref().expect("key required for deleted file"),
            )
            .with_context(|| format!("failed to decrypt baseline blob for {path}"))?;

            output::print_diff_header(path);
            if is_binary(baseline.expose_secret()) {
                output::print_binary_modified(path, baseline.expose_secret().len(), 0);
                continue;
            }

            let old_text = String::from_utf8_lossy(baseline.expose_secret());
            let rendered = unified_diff(&old_text, "", &format!("a/{path}"), &format!("b/{path}"));
            output::print_diff_text(&rendered);
        }
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
