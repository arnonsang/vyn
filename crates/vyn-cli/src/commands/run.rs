use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use secrecy::ExposeSecret;
use serde::Deserialize;
use vyn_core::blob::decrypt_blob_by_hash;
use vyn_core::keychain::load_project_key;
use vyn_core::manifest::Manifest;

#[derive(Debug, Deserialize)]
struct VaultConfig {
    vault_id: String,
}

pub fn run(cmd: Vec<String>) -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;

    let env_map = collect_env_map(&root)?;
    if env_map.is_empty() {
        anyhow::bail!("no env variables discovered (.env files or encrypted env blobs)");
    }

    let program = cmd
        .first()
        .with_context(|| "no command provided to run")?
        .to_string();
    let args = cmd.into_iter().skip(1).collect::<Vec<_>>();

    let status = std::process::Command::new(program)
        .args(args)
        .envs(env_map)
        .status()
        .context("failed to execute child process")?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        anyhow::bail!("child process exited with non-zero status: {code}");
    }

    Ok(())
}

fn collect_env_map(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut env_map = BTreeMap::new();

    let (manifest, key) = load_manifest_and_key(root);
    if let (Some(manifest), Some(key)) = (manifest, key) {
        let blobs_dir = root.join(".vyn").join("blobs");
        for entry in manifest.files.iter().filter(|f| is_env_path(&f.path)) {
            if let Ok(secret) = decrypt_blob_by_hash(&blobs_dir, &entry.sha256, &key)
                && let Ok(content) = std::str::from_utf8(secret.expose_secret())
            {
                merge_dotenv(&mut env_map, content);
            }
        }
    }

    for path in discover_local_env_files(root)? {
        if let Ok(content) = fs::read_to_string(&path) {
            merge_dotenv(&mut env_map, &content);
        }
    }

    Ok(env_map)
}

fn load_manifest_and_key(root: &Path) -> (Option<Manifest>, Option<vyn_core::crypto::SecretBytes>) {
    let config_path = root.join(".vyn").join("config.toml");
    let manifest_path = root.join(".vyn").join("manifest.json");

    let config = fs::read_to_string(config_path)
        .ok()
        .and_then(|text| toml::from_str::<VaultConfig>(&text).ok());
    let manifest = fs::read_to_string(manifest_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Manifest>(&text).ok());

    if let (Some(config), Some(manifest)) = (config, manifest) {
        let key = load_project_key(&config.vault_id).ok();
        return (Some(manifest), key);
    }

    (None, None)
}

fn discover_local_env_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();

    let direct = root.join(".env");
    if direct.exists() && direct.is_file() {
        out.push(direct);
    }

    for entry in fs::read_dir(root).context("failed to scan project directory")? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(OsStr::to_str)
            && name.starts_with(".env.")
            && name != ".env.example"
        {
            out.push(path);
        }
    }

    out.sort();
    Ok(out)
}

fn is_env_path(path: &str) -> bool {
    let file_name = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();

    file_name == ".env" || file_name.starts_with(".env.")
}

fn merge_dotenv(map: &mut BTreeMap<String, String>, content: &str) {
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        let value = value.trim();
        let value = strip_quotes(value);
        map.insert(key.to_string(), value.to_string());
    }
}

fn strip_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}
