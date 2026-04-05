use std::collections::HashMap;
use std::sync::Arc;

use secrecy::ExposeSecret;
use tokio::sync::RwLock;

use crate::crypto::{SecretBytes, decrypt, encrypt, secret_bytes};
use crate::manifest::Manifest;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("object not found")]
    NotFound,
    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("crypto operation failed: {0}")]
    Crypto(#[from] crate::crypto::CryptoError),
    #[error("transport error: {0}")]
    Transport(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[allow(async_fn_in_trait)]
pub trait StorageProvider: Send + Sync {
    async fn get_manifest(&self, project_id: &str) -> StorageResult<Option<Vec<u8>>>;
    async fn put_manifest(&self, project_id: &str, manifest_payload: &[u8]) -> StorageResult<()>;
    async fn upload_blob(&self, hash: &str, data: Vec<u8>) -> StorageResult<()>;
    async fn download_blob(&self, hash: &str) -> StorageResult<Option<Vec<u8>>>;
    async fn create_invite(
        &self,
        user_id: &str,
        vault_id: &str,
        payload: Vec<u8>,
    ) -> StorageResult<()>;
    async fn get_invites(&self, user_id: &str, vault_id: &str) -> StorageResult<Vec<Vec<u8>>>;
}

#[derive(Clone, Default)]
pub struct InMemoryStorageProvider {
    manifests: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    blobs: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    invites: Arc<RwLock<HashMap<String, Vec<Vec<u8>>>>>,
}

impl InMemoryStorageProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StorageProvider for InMemoryStorageProvider {
    async fn get_manifest(&self, project_id: &str) -> StorageResult<Option<Vec<u8>>> {
        Ok(self.manifests.read().await.get(project_id).cloned())
    }

    async fn put_manifest(&self, project_id: &str, manifest_payload: &[u8]) -> StorageResult<()> {
        self.manifests
            .write()
            .await
            .insert(project_id.to_string(), manifest_payload.to_vec());
        Ok(())
    }

    async fn upload_blob(&self, hash: &str, data: Vec<u8>) -> StorageResult<()> {
        self.blobs.write().await.insert(hash.to_string(), data);
        Ok(())
    }

    async fn download_blob(&self, hash: &str) -> StorageResult<Option<Vec<u8>>> {
        Ok(self.blobs.read().await.get(hash).cloned())
    }

    async fn create_invite(
        &self,
        user_id: &str,
        vault_id: &str,
        payload: Vec<u8>,
    ) -> StorageResult<()> {
        let key = format!("{user_id}:{vault_id}");
        let mut invites = self.invites.write().await;
        invites.entry(key).or_default().push(payload);
        Ok(())
    }

    async fn get_invites(&self, user_id: &str, vault_id: &str) -> StorageResult<Vec<Vec<u8>>> {
        let key = format!("{user_id}:{vault_id}");
        Ok(self
            .invites
            .read()
            .await
            .get(&key)
            .cloned()
            .unwrap_or_default())
    }
}

pub fn encrypt_manifest(manifest: &Manifest, key: &SecretBytes) -> StorageResult<Vec<u8>> {
    let bytes = serde_json::to_vec(manifest)?;
    let encrypted = encrypt(key, &secret_bytes(bytes))?;

    let mut payload = Vec::with_capacity(encrypted.nonce.len() + encrypted.ciphertext.len());
    payload.extend_from_slice(&encrypted.nonce);
    payload.extend_from_slice(&encrypted.ciphertext);
    Ok(payload)
}

pub fn decrypt_manifest(payload: &[u8], key: &SecretBytes) -> StorageResult<Manifest> {
    const NONCE_LEN: usize = 12;
    if payload.len() < NONCE_LEN {
        return Err(StorageError::NotFound);
    }

    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&payload[..NONCE_LEN]);
    let encrypted = crate::crypto::EncryptedData {
        nonce,
        ciphertext: payload[NONCE_LEN..].to_vec(),
    };

    let plaintext = decrypt(key, &encrypted)?;
    Ok(serde_json::from_slice(plaintext.expose_secret())?)
}

#[cfg(test)]
mod tests {
    use super::{InMemoryStorageProvider, StorageProvider, decrypt_manifest, encrypt_manifest};
    use crate::crypto::secret_bytes;
    use crate::manifest::{FileEntry, Manifest};

    #[test]
    fn manifest_encryption_roundtrip() {
        let manifest = Manifest {
            version: 1,
            files: vec![FileEntry {
                path: ".env".to_string(),
                sha256: "abc".to_string(),
                size: 42,
                mode: 0o644,
            }],
        };
        let key = secret_bytes(vec![7u8; 32]);

        let encrypted =
            encrypt_manifest(&manifest, &key).expect("manifest encryption should succeed");
        let restored =
            decrypt_manifest(&encrypted, &key).expect("manifest decryption should succeed");

        assert_eq!(manifest, restored);
    }

    #[tokio::test]
    async fn push_pull_roundtrip() {
        let provider = InMemoryStorageProvider::new();
        let key = secret_bytes(vec![9u8; 32]);
        let project = "vault-123";

        let manifest = Manifest {
            version: 1,
            files: vec![FileEntry {
                path: ".env".to_string(),
                sha256: "hash-1".to_string(),
                size: 5,
                mode: 0o644,
            }],
        };

        let manifest_payload = encrypt_manifest(&manifest, &key).expect("manifest should encrypt");
        provider
            .put_manifest(project, &manifest_payload)
            .await
            .expect("manifest should upload");
        provider
            .upload_blob("hash-1", b"hello".to_vec())
            .await
            .expect("blob should upload");

        let pulled_manifest_payload = provider
            .get_manifest(project)
            .await
            .expect("manifest should download")
            .expect("manifest should exist");
        let pulled_manifest =
            decrypt_manifest(&pulled_manifest_payload, &key).expect("manifest should decrypt");
        let blob = provider
            .download_blob("hash-1")
            .await
            .expect("blob should download")
            .expect("blob should exist");

        assert_eq!(pulled_manifest, manifest);
        assert_eq!(blob, b"hello");
    }
}
