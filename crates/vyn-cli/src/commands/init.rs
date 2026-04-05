use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use console::style;
use dialoguer::MultiSelect;
use ignore::gitignore::GitignoreBuilder;
use uuid::Uuid;
use vyn_core::crypto::generate_project_key;
use vyn_core::ignore::load_ignore_matcher;
use vyn_core::keychain::store_project_key;
use vyn_core::manifest::capture_manifest;
use walkdir::WalkDir;

use crate::output;


/// Patterns that strongly suggest secrets / config (candidate for vyn tracking).
const SECRET_HINTS: &[&str] = &[
    ".env", "secret", "credential", "passwd", "password", "token",
    ".pem", ".key", ".p12", ".pfx", ".crt", ".cert", "id_rsa", "id_ed25519",
    "config", "settings", "keystore", ".kubeconfig", ".kube",
];

/// Patterns that strongly suggest build artifacts (should stay ignored by vyn).
const ARTIFACT_HINTS: &[&str] = &[
    "node_modules", "target", "dist", "build", "vendor", "__pycache__",
    ".cache", ".gradle", ".m2", "*.o", "*.class", "*.pyc", "*.log",
    "*.lock", "Thumbs.db", ".DS_Store",
];

fn looks_like_secret(pattern: &str) -> bool {
    let lower = pattern.to_lowercase();
    SECRET_HINTS.iter().any(|h| lower.contains(h))
        && !ARTIFACT_HINTS.iter().any(|h| lower.contains(h))
}

fn is_artifact(pattern: &str) -> bool {
    let lower = pattern.to_lowercase();
    ARTIFACT_HINTS.iter().any(|h| lower.contains(h))
}

fn make_row(pattern: &str, files: &[String]) -> String {
    let preview = if files.len() <= 3 {
        files.join(", ")
    } else {
        format!("{}, {} … (+{})", files[0], files[1], files.len() - 2)
    };
    format!("{:<30}  {}", style(pattern).yellow(), style(&preview).dim())
}

/// Find files in `root` that are matched by `.gitignore` pattern `pattern`.
fn files_matched_by_pattern(root: &Path, pattern: &str) -> Vec<String> {
    let mut builder = GitignoreBuilder::new(root);
    builder.add_line(None, pattern).ok();
    let Ok(gi) = builder.build() else { return vec![] };

    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            // skip .git and .vyn
            if p.components().any(|c| {
                matches!(c.as_os_str().to_str(), Some(".git") | Some(".vyn"))
            }) {
                return false;
            }
            let is_dir = e.file_type().is_dir();
            gi.matched(p.strip_prefix(root).unwrap_or(p), is_dir)
                .is_ignore()
        })
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            e.path()
                .strip_prefix(root)
                .ok()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string())
        })
        .collect()
}

pub fn run(name: Option<String>) -> Result<()> {
    output::print_banner("init");
    let root = std::env::current_dir().context("failed to determine current directory")?;

    let vault_dir = root.join(".vyn");
    if vault_dir.join("config.toml").exists() {
        anyhow::bail!(
            "vault already initialized in this directory (.vyn/config.toml exists). \
             Run `vyn st` to check status or `vyn doctor` to diagnose issues."
        );
    }

    let project_name = name.unwrap_or_else(|| {
        root.file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("vyn-project")
            .to_string()
    });

    let spinner = output::new_spinner("generating project key…");
    let vault_id = Uuid::new_v4().to_string();
    let project_key = generate_project_key().context("failed to generate project key")?;
    store_project_key(&vault_id, &project_key)
        .context("failed to store project key in keychain")?;
    output::finish_progress(&spinner, "project key generated and stored in keychain");

    let blobs_dir = vault_dir.join("blobs");
    fs::create_dir_all(&blobs_dir).context("failed to create vault directories")?;

    let vynignore_path = root.join(".vynignore");
    if !vynignore_path.exists() {
        write_vynignore_interactive(&root, &vynignore_path)?;
    }

    let spinner2 = output::new_spinner("scanning files…");
    let matcher = load_ignore_matcher(&root).context("failed to load ignore matcher")?;
    let manifest = capture_manifest(&root, &matcher).context("failed to capture local manifest")?;
    output::finish_progress(&spinner2, &format!("{} files indexed", manifest.files.len()));

    let manifest_path = vault_dir.join("manifest.json");
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("failed to serialize manifest")?;
    fs::write(&manifest_path, manifest_json).context("failed to write manifest.json")?;

    let config_path = vault_dir.join("config.toml");
    let config = format!(
        "vault_id = \"{vault_id}\"\nproject_name = \"{project_name}\"\nstorage_provider = \"unconfigured\"\n"
    );
    fs::write(&config_path, config).context("failed to write config.toml")?;

    ensure_gitignore_contains_vyn(&root)?;

    output::print_success(&format!("vault '{project_name}' initialized"));
    output::print_info("vault id", &vault_id);
    output::print_info("tracked files", &manifest.files.len().to_string());
    output::print_info("storage", "unconfigured  (run vyn config to set up)");
    println!();
    Ok(())
}

/// Build `.vynignore` interactively:
///  1. Read `.gitignore` patterns.
///  2. Classify each pattern as likely-secret or likely-artifact.
///  3. Find which of those patterns actually match files on disk.
///  4. Show candidates grouped, let user confirm via multi-select.
///  5. Write `.vynignore` = everything NOT selected (i.e. selected files are tracked by vyn; everything else is excluded).
fn write_vynignore_interactive(root: &Path, vynignore_path: &Path) -> Result<()> {
    let gitignore_path = root.join(".gitignore");
    if !gitignore_path.exists() {
        // No .gitignore: write a minimal default and return.
        fs::write(vynignore_path, "target/\nnode_modules/\n.git/\n")
            .context("failed to write .vynignore")?;
        return Ok(());
    }

    let gitignore_content =
        fs::read_to_string(&gitignore_path).context("failed to read .gitignore")?;

    // Collect patterns, skip comments/blanks.
    let patterns: Vec<&str> = gitignore_content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    if patterns.is_empty() {
        fs::write(vynignore_path, "target/\nnode_modules/\n.git/\n")
            .context("failed to write .vynignore")?;
        return Ok(());
    }

    // Split patterns into: secret candidates, other git-ignored (non-artifact).
    let mut secret_candidates: Vec<(String, Vec<String>)> = Vec::new();
    let mut other_candidates: Vec<(String, Vec<String>)> = Vec::new();

    for pattern in &patterns {
        let files = files_matched_by_pattern(root, pattern);
        if files.is_empty() {
            continue;
        }
        if looks_like_secret(pattern) {
            secret_candidates.push((pattern.to_string(), files));
        } else if !is_artifact(pattern) {
            other_candidates.push((pattern.to_string(), files));
        }
    }

    // Artifact patterns are always excluded, no prompt needed.
    let artifact_patterns: Vec<&str> = patterns
        .iter()
        .copied()
        .filter(|p| is_artifact(p))
        .collect();

    println!();
    println!(
        "  {} Scanning git-ignored files…",
        style("[vyn]").cyan().bold(),
    );

    let hint = style("(Space to toggle, Enter to confirm, ↑↓ to move)").dim();

    let mut tracked_patterns: Vec<String> = Vec::new();

    if !secret_candidates.is_empty() {
        println!();
        println!(
            "  {} Found {} pattern(s) that look like secrets: select which to {} with vyn:",
            style("→").cyan().bold(),
            secret_candidates.len(),
            style("track and encrypt").green().bold()
        );
        println!("  {hint}");
        println!();

        let items: Vec<String> = secret_candidates.iter().map(|(p, files)| make_row(p, files)).collect();
        let defaults = vec![true; items.len()];

        let sel = MultiSelect::new()
            .with_prompt("  Secrets to track")
            .items(&items)
            .defaults(&defaults)
            .interact()
            .context("selection cancelled")?;

        for i in sel {
            tracked_patterns.push(secret_candidates[i].0.clone());
        }
    }

    if !other_candidates.is_empty() {
        println!();
        println!(
            "  {} {} other git-ignored file(s) found, select any you also want vyn to track:",
            style("→").cyan().bold(),
            other_candidates.len(),
        );
        println!("  {hint}");
        println!();

        let items: Vec<String> = other_candidates.iter().map(|(p, files)| make_row(p, files)).collect();
        let defaults = vec![false; items.len()];

        let sel = MultiSelect::new()
            .with_prompt("  Also track")
            .items(&items)
            .defaults(&defaults)
            .interact()
            .context("selection cancelled")?;

        for i in sel {
            tracked_patterns.push(other_candidates[i].0.clone());
        }
    }

    if secret_candidates.is_empty() && other_candidates.is_empty() {
        println!("  No git-ignored files detected on disk. Excluding all gitignore patterns.");
        println!();
        let content = build_vynignore_content(&patterns, &[]);
        fs::write(vynignore_path, content).context("failed to write .vynignore")?;
        return Ok(());
    }

    // Patterns to exclude = all gitignore patterns minus what the user wants to track.
    let excluded_patterns: Vec<&str> = patterns
        .iter()
        .copied()
        .filter(|p| !tracked_patterns.iter().any(|t| t == p))
        .collect();

    let content = build_vynignore_content(&excluded_patterns, &artifact_patterns);
    fs::write(vynignore_path, &content).context("failed to write .vynignore")?;

    println!();
    if tracked_patterns.is_empty() {
        output::print_info(".vynignore", "all git-ignored files excluded (nothing tracked)");
    } else {
        output::print_info(
            ".vynignore",
            &format!(
                "{} pattern(s) will be tracked and encrypted by vyn",
                tracked_patterns.len()
            ),
        );
        for p in &tracked_patterns {
            println!("    {} {}", style("+").green(), p);
        }
    }
    println!();

    Ok(())
}

/// Build the `.vynignore` file content from the list of patterns to exclude.
/// Always prepends the mandatory hardcoded exclusions.
fn build_vynignore_content(excluded: &[&str], _artifact_patterns: &[&str]) -> String {
    let mut lines = vec![
        "# Generated by vyn init".to_string(),
        "# Patterns listed here are excluded from vyn tracking.".to_string(),
        "# Remove a pattern to start tracking those files.".to_string(),
        String::new(),
        "# vyn internals".to_string(),
        ".vyn/".to_string(),
        ".git/".to_string(),
        String::new(),
        "# Excluded patterns (from .gitignore)".to_string(),
    ];

    for p in excluded {
        lines.push(p.to_string());
    }

    lines.push(String::new());
    lines.join("\n")
}

fn ensure_gitignore_contains_vyn(root: &Path) -> Result<()> {
    let path = root.join(".gitignore");
    let mut content = if path.exists() {
        fs::read_to_string(&path).context("failed to read .gitignore")?
    } else {
        String::new()
    };

    if !content.lines().any(|line| line.trim() == ".vyn/") {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(".vyn/\n");
        fs::write(path, content).context("failed to update .gitignore")?;
    }

    Ok(())
}
