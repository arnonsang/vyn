use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::output;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    timestamp_unix: u64,
    source: String,
    manifest_version: u64,
    file_count: usize,
}

pub fn run() -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;
    let history_dir = root.join(".vyn").join("history");

    output::print_banner("history");
    if !history_dir.exists() {
        output::print_warning("no history found");
        println!();
        return Ok(());
    }

    let mut entries = Vec::new();
    for dir_entry in fs::read_dir(&history_dir)
        .with_context(|| format!("failed to read {}", history_dir.display()))?
    {
        let path = dir_entry?.path();
        if !path.is_file() {
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&text) {
            entries.push(entry);
        }
    }

    entries.sort_by_key(|e| std::cmp::Reverse(e.timestamp_unix));

    if entries.is_empty() {
        output::print_warning("no history found");
        println!();
        return Ok(());
    }

    let mut table = output::make_table(&["time (unix)", "action", "manifest ver", "files"]);
    for entry in &entries {
        table.add_row(vec![
            entry.timestamp_unix.to_string(),
            entry.source.clone(),
            entry.manifest_version.to_string(),
            entry.file_count.to_string(),
        ]);
    }
    output::print_table(&table);
    Ok(())
}

pub fn write_history_entry(
    root: &Path,
    source: &str,
    manifest_version: u64,
    file_count: usize,
) -> Result<()> {
    let history_dir = root.join(".vyn").join("history");
    fs::create_dir_all(&history_dir)
        .with_context(|| format!("failed to create {}", history_dir.display()))?;

    let ts = now_unix_seconds();
    let pid = std::process::id();
    let file_name = format!("{}-{}-{}.json", ts, source, pid);
    let path = history_dir.join(file_name);

    let record = HistoryEntry {
        timestamp_unix: ts,
        source: source.to_string(),
        manifest_version,
        file_count,
    };

    fs::write(
        &path,
        serde_json::to_string_pretty(&record).context("failed to serialize history entry")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
