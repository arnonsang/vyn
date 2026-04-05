use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub fn run() -> Result<()> {
    let root = std::env::current_dir().context("failed to determine current directory")?;

    let env_path = root.join(".env");
    let example_path = root.join(".env.example");

    if !env_path.exists() {
        anyhow::bail!("missing .env");
    }
    if !example_path.exists() {
        anyhow::bail!("missing .env.example");
    }

    let env_keys = parse_env_keys(&env_path)?;
    let example_keys = parse_env_keys(&example_path)?;

    let missing = example_keys
        .difference(&env_keys)
        .cloned()
        .collect::<Vec<_>>();
    let extra = env_keys
        .difference(&example_keys)
        .cloned()
        .collect::<Vec<_>>();

    if missing.is_empty() && extra.is_empty() {
        println!("check passed: .env matches .env.example keys");
        return Ok(());
    }

    if !missing.is_empty() {
        println!("missing keys in .env:");
        for key in &missing {
            println!("  - {key}");
        }
    }

    if !extra.is_empty() {
        println!("extra keys in .env:");
        for key in &extra {
            println!("  - {key}");
        }
    }

    anyhow::bail!("check failed: key mismatch between .env and .env.example")
}

fn parse_env_keys(path: &Path) -> Result<BTreeSet<String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    let mut keys = BTreeSet::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if !key.is_empty() {
            keys.insert(key.to_string());
        }
    }

    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::parse_env_keys;
    use std::fs;

    #[test]
    fn parse_env_keys_ignores_comments_and_blank_lines() {
        let tmp = std::env::temp_dir().join(format!("vyn-check-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).expect("temp dir should be created");
        let file = tmp.join(".env.example");
        fs::write(
            &file,
            "\n# comment\nexport API_KEY=abc\nDATABASE_URL=postgres://local\nINVALID_LINE\n",
        )
        .expect("fixture should be written");

        let keys = parse_env_keys(&file).expect("parser should succeed");
        assert!(keys.contains("API_KEY"));
        assert!(keys.contains("DATABASE_URL"));
        assert_eq!(keys.len(), 2);

        fs::remove_dir_all(tmp).expect("temp dir should be removed");
    }
}
