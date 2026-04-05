use crate::ignore::IgnoreMatcher;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub version: u64,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to walk directory: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("failed to read file metadata: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid relative path")]
    InvalidPath,
}

impl Manifest {
    pub fn empty() -> Self {
        Self {
            version: 1,
            files: Vec::new(),
        }
    }
}

pub fn capture_manifest(root: &Path, matcher: &IgnoreMatcher) -> Result<Manifest, ManifestError> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        let is_dir = entry.file_type().is_dir();

        if matcher.should_ignore(path, is_dir) {
            continue;
        }

        if is_dir {
            continue;
        }

        let rel_path = to_relative_path(root, path)?;
        let metadata = fs::metadata(path)?;
        let data = fs::read(path)?;

        files.push(FileEntry {
            path: rel_path,
            sha256: sha256_hex(&data),
            size: metadata.len(),
            mode: file_mode(&metadata),
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(Manifest { version: 1, files })
}

fn to_relative_path(root: &Path, path: &Path) -> Result<String, ManifestError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| ManifestError::InvalidPath)?;
    Ok(path_to_unix(rel))
}

fn path_to_unix(path: &Path) -> String {
    let mut out = PathBuf::new();
    for part in path.components() {
        out.push(part.as_os_str());
    }
    out.to_string_lossy().replace('\\', "/")
}

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut output = String::with_capacity(digest.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";

    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

#[cfg(unix)]
fn file_mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn file_mode(_metadata: &std::fs::Metadata) -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::capture_manifest;
    use crate::ignore::load_ignore_matcher;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn manifest_integrity() {
        let tmp = std::env::temp_dir().join(format!("vyn-manifest-{}", Uuid::new_v4()));
        fs::create_dir_all(tmp.join("dir")).expect("test directories should be created");
        fs::write(tmp.join("dir").join("a.env"), "A=1\nB=2\n")
            .expect("file a.env should be written");
        fs::write(tmp.join("b.yaml"), "name: vyn\n").expect("file b.yaml should be written");

        let matcher = load_ignore_matcher(&tmp).expect("matcher should load");
        let manifest = capture_manifest(&tmp, &matcher).expect("manifest capture should succeed");

        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.files[0].path, "b.yaml");
        assert_eq!(manifest.files[1].path, "dir/a.env");
        assert!(manifest.files.iter().all(|f| f.size > 0));

        fs::remove_dir_all(tmp).expect("temp directory should be removed");
    }
}
