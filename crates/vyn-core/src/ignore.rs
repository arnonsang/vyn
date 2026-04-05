use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug)]
pub struct IgnoreMatcher {
    root: PathBuf,
    gitignore: Gitignore,
}

#[derive(Debug, Error)]
pub enum IgnoreError {
    #[error("failed to build ignore matcher")]
    BuildFailure,
}

impl IgnoreMatcher {
    pub fn should_ignore(&self, path: &Path, is_dir: bool) -> bool {
        if let Ok(rel_path) = path.strip_prefix(&self.root) {
            // Always ignore vyn internals and git internals.
            if rel_path == Path::new(".vyn") || rel_path.starts_with(Path::new(".vyn")) {
                return true;
            }
            if rel_path == Path::new(".git") || rel_path.starts_with(Path::new(".git")) {
                return true;
            }

            return self.gitignore.matched(rel_path, is_dir).is_ignore();
        }

        true
    }
}

pub fn load_ignore_matcher(root: &Path) -> Result<IgnoreMatcher, IgnoreError> {
    let mut builder = GitignoreBuilder::new(root);
    let ignore_file = root.join(".vynignore");
    if ignore_file.exists() {
        builder.add(ignore_file);
    }

    let gitignore = builder.build().map_err(|_| IgnoreError::BuildFailure)?;

    Ok(IgnoreMatcher {
        root: root.to_path_buf(),
        gitignore,
    })
}

#[cfg(test)]
mod tests {
    use super::load_ignore_matcher;
    use std::fs;
    use std::path::Path;
    use uuid::Uuid;

    #[test]
    fn ignore_patterns() {
        let tmp = std::env::temp_dir().join(format!("vyn-ignore-{}", Uuid::new_v4()));
        fs::create_dir_all(&tmp).expect("temp dir should be created");
        fs::write(tmp.join(".vynignore"), "*.tmp\nsecrets/**\n")
            .expect("ignore file should be written");
        fs::create_dir_all(tmp.join("secrets")).expect("secrets directory should be created");
        fs::write(tmp.join("secrets").join("token.env"), "TOKEN=abc\n")
            .expect("secrets token file should be written");

        let matcher = load_ignore_matcher(&tmp).expect("matcher should load");
        assert!(matcher.should_ignore(&tmp.join("foo.tmp"), false));
        assert!(matcher.should_ignore(&tmp.join("secrets").join("token.env"), false));
        assert!(!matcher.should_ignore(&tmp.join("app.env"), false));
        assert!(matcher.should_ignore(&tmp.join(".vyn").join("manifest.json"), false));
        assert!(matcher.should_ignore(&tmp.join(".git").join("config"), false));

        fs::remove_dir_all(Path::new(&tmp)).expect("temp dir should be removed");
    }
}
