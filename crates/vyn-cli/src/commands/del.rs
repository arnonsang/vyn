use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use console::style;
use vyn_core::ignore::load_ignore_matcher;
use vyn_core::manifest::capture_manifest;

use crate::output;

pub fn run(paths: Vec<String>) -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let vault_dir = root.join(".vyn");

    if !vault_dir.join("config.toml").exists() {
        anyhow::bail!("no vault found in current directory: run `vyn init` first");
    }

    let vynignore_path = root.join(".vynignore");
    let original = if vynignore_path.exists() {
        fs::read_to_string(&vynignore_path).context("failed to read .vynignore")?
    } else {
        String::new()
    };

    let mut to_append: Vec<String> = Vec::new();

    for raw in &paths {
        let path = Path::new(raw);
        let rel = if path.is_absolute() {
            path.strip_prefix(&root)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| raw.clone())
        } else {
            raw.clone()
        };

        // Check it actually exists (warn but still allow patterns like *.log).
        if !root.join(&rel).exists() {
            println!(
                "  {} {}: file not found (pattern will still be added to .vynignore)",
                style("!").yellow(),
                raw
            );
        }

        // Check if an identical line already exists.
        if original.lines().any(|l| l.trim() == rel.trim()) {
            println!(
                "  {} {}: already in .vynignore, skipping",
                style("✓").dim(),
                raw
            );
            continue;
        }

        println!(
            "  {} {}: will no longer be tracked",
            style("-").red().bold(),
            raw
        );
        to_append.push(rel);
    }

    if to_append.is_empty() {
        return Ok(());
    }

    // Append new patterns to .vynignore.
    let mut content = original;
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str("\n# Manually untracked\n");
    for p in &to_append {
        content.push_str(p);
        content.push('\n');
    }

    fs::write(&vynignore_path, content).context("failed to write .vynignore")?;

    // Refresh manifest.
    let spinner = output::new_spinner("refreshing manifest…");
    let matcher = load_ignore_matcher(&root).context("failed to reload ignore matcher")?;
    let manifest = capture_manifest(&root, &matcher).context("failed to capture manifest")?;
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?;
    fs::write(vault_dir.join("manifest.json"), manifest_json)
        .context("failed to write manifest.json")?;
    output::finish_progress(&spinner, &format!("{} files now tracked", manifest.files.len()));
    println!(
        "  {} run {} to sync changes to remote",
        style("hint:").dim(),
        style("vyn push").cyan()
    );

    Ok(())
}
