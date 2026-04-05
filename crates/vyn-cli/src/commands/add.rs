use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::style;
use ignore::gitignore::GitignoreBuilder;
use vyn_core::ignore::load_ignore_matcher;
use vyn_core::manifest::capture_manifest;

use crate::output;

pub fn run(paths: Vec<String>) -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let vault_dir = root.join(".vyn");

    if !vault_dir.join("config.toml").exists() {
        anyhow::bail!("no vault found in current directory,  run `vyn init` first");
    }

    let vynignore_path = root.join(".vynignore");
    let original = if vynignore_path.exists() {
        fs::read_to_string(&vynignore_path).context("failed to read .vynignore")?
    } else {
        String::new()
    };

    let mut changed_any = false;

    for raw in &paths {
        // Resolve the path relative to root so we can test it against patterns.
        let target: PathBuf = if Path::new(raw).is_absolute() {
            Path::new(raw)
                .strip_prefix(&root)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| PathBuf::from(raw))
        } else {
            PathBuf::from(raw)
        };

        let abs_target = root.join(&target);
        if !abs_target.exists() {
            println!(
                "  {} {}: file not found, skipping",
                style("!").yellow(),
                raw
            );
            continue;
        }

        let is_dir = abs_target.is_dir();

        // Find which lines in .vynignore cause this path to be ignored.
        let lines: Vec<&str> = original.lines().collect();
        let mut culprits: Vec<usize> = Vec::new(); // line indices to remove

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut builder = GitignoreBuilder::new(&root);
            builder.add_line(None, trimmed).ok();
            if let Ok(gi) = builder.build()
                && gi.matched(&target, is_dir).is_ignore()
            {
                culprits.push(i);
            }
        }

        if culprits.is_empty() {
            println!(
                "  {} {}: not currently ignored, already tracked",
                style("✓").green(),
                raw
            );
        } else {
            let removed_patterns: Vec<&str> = culprits.iter().map(|&i| lines[i]).collect();
            println!(
                "  {} {}: removing {} ignore pattern(s): {}",
                style("+").green().bold(),
                raw,
                culprits.len(),
                removed_patterns.join(", ")
            );
            changed_any = true;
        }
    }

    if !changed_any {
        return Ok(());
    }

    // Rebuild .vynignore without the culprit lines.
    // Re-compute which lines to keep across all paths in one pass.
    let lines: Vec<&str> = original.lines().collect();
    let mut remove_indices = std::collections::HashSet::new();

    for raw in &paths {
        let target: PathBuf = if Path::new(raw).is_absolute() {
            Path::new(raw)
                .strip_prefix(&root)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| PathBuf::from(raw))
        } else {
            PathBuf::from(raw)
        };
        let abs_target = root.join(&target);
        if !abs_target.exists() {
            continue;
        }
        let is_dir = abs_target.is_dir();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut builder = GitignoreBuilder::new(&root);
            builder.add_line(None, trimmed).ok();
            if let Ok(gi) = builder.build()
                && gi.matched(&target, is_dir).is_ignore()
            {
                remove_indices.insert(i);
            }
        }
    }

    let new_content: String = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !remove_indices.contains(i))
        .map(|(_, l)| *l)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    fs::write(&vynignore_path, new_content).context("failed to write .vynignore")?;

    // Refresh manifest.
    let spinner = output::new_spinner("refreshing manifest…");
    let matcher = load_ignore_matcher(&root).context("failed to reload ignore matcher")?;
    let manifest = capture_manifest(&root, &matcher).context("failed to capture manifest")?;
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?;
    fs::write(vault_dir.join("manifest.json"), manifest_json)
        .context("failed to write manifest.json")?;
    output::finish_progress(
        &spinner,
        &format!("{} files now tracked", manifest.files.len()),
    );
    println!(
        "  {} run {} to sync changes to remote",
        style("hint:").dim(),
        style("vyn push").cyan()
    );

    Ok(())
}
