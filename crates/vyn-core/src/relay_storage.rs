use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use tonic::Request;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};

use crate::storage::{StorageError, StorageProvider, StorageResult};

#[derive(Clone)]
pub struct RelayStorageProvider {
    endpoint: String,
    pub token: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl RelayStorageProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    async fn connect(
        &self,
    ) -> StorageResult<vyn_relay::proto::vyn_relay_client::VynRelayClient<Channel>> {
        let endpoint = Endpoint::from_shared(self.endpoint.clone())
            .map_err(|err| StorageError::Transport(err.to_string()))?;
        vyn_relay::proto::vyn_relay_client::VynRelayClient::connect(endpoint)
            .await
            .map_err(|err| StorageError::Transport(err.to_string()))
    }

    /// Perform the two-step challenge-response auth and store the session token.
    pub async fn authenticate(
        &self,
        user_id: &str,
        sign_fn: impl Fn(&[u8]) -> StorageResult<String>,
    ) -> StorageResult<()> {
        let mut client = self.connect().await?;

        // Step 1: request challenge
        let challenge_resp = client
            .authenticate(Request::new(vyn_relay::proto::AuthRequest {
                user_id: user_id.to_string(),
                nonce: Vec::new(),
                signature: String::new(),
            }))
            .await
            .map_err(|e| StorageError::Transport(e.to_string()))?
            .into_inner();

        let nonce = challenge_resp.challenge_nonce;

        // Step 2: sign nonce and send back
        let signature = sign_fn(&nonce)?;
        let auth_resp = client
            .authenticate(Request::new(vyn_relay::proto::AuthRequest {
                user_id: user_id.to_string(),
                nonce: nonce.clone(),
                signature,
            }))
            .await
            .map_err(|e| StorageError::Transport(e.to_string()))?
            .into_inner();

        if !auth_resp.authenticated {
            return Err(StorageError::Transport(
                "relay authentication failed".to_string(),
            ));
        }

        *self.token.write().await = Some(auth_resp.token);
        Ok(())
    }

    /// Register identity on the relay (idempotent) then authenticate.
    /// `vault_dir` is the `.vyn/` directory (contains `identity.toml`).
    pub async fn authenticate_with_identity(&self, vault_dir: &Path) -> StorageResult<()> {
        let identity = load_identity(vault_dir)?;
        let private_key_path = identity.ssh_private_key.clone();
        let public_key_path = identity.ssh_public_key.clone();
        let user_id = identity.github_username.clone();

        let public_key = std::fs::read_to_string(&public_key_path)
            .map_err(|e| StorageError::Transport(format!("failed to read public key: {e}")))?;
        let public_key = public_key.trim().to_string();

        // Register identity on the relay (no-op if already registered with same key)
        self.ensure_identity_registered(&user_id, &public_key, &private_key_path)
            .await?;

        let private_key_path2 = private_key_path.clone();
        self.authenticate(&user_id, move |nonce| {
            sign_nonce_with_ssh_key(nonce, Path::new(&private_key_path2))
        })
        .await
    }

    async fn ensure_identity_registered(
        &self,
        user_id: &str,
        public_key: &str,
        private_key_path: &str,
    ) -> StorageResult<()> {
        let registration_payload = format!("vyn-register:{user_id}:{public_key}");
        let signature =
            sign_nonce_with_ssh_key(registration_payload.as_bytes(), Path::new(private_key_path))?;

        let mut client = self.connect().await?;
        client
            .register_identity(Request::new(vyn_relay::proto::RegisterRequest {
                user_id: user_id.to_string(),
                public_key: public_key.to_string(),
                signature,
            }))
            .await
            .map_err(|e| StorageError::Transport(e.to_string()))?;

        Ok(())
    }

    async fn inject_token<T>(&self, mut request: Request<T>) -> StorageResult<Request<T>> {
        if let Some(ref tok) = *self.token.read().await {
            let val = MetadataValue::try_from(tok.as_str())
                .map_err(|e| StorageError::Transport(e.to_string()))?;
            request.metadata_mut().insert("x-vyn-token", val);
        }
        Ok(request)
    }
}

impl StorageProvider for RelayStorageProvider {
    async fn get_manifest(&self, project_id: &str) -> StorageResult<Option<Vec<u8>>> {
        let mut client = self.connect().await?;
        let response = client
            .get_manifest(Request::new(vyn_relay::proto::GetManifestRequest {
                project_id: project_id.to_string(),
            }))
            .await
            .map_err(|err| StorageError::Transport(err.to_string()))?
            .into_inner();

        if response.found {
            Ok(Some(response.payload))
        } else {
            Ok(None)
        }
    }

    async fn put_manifest(&self, project_id: &str, manifest_payload: &[u8]) -> StorageResult<()> {
        let mut client = self.connect().await?;
        let req = self
            .inject_token(Request::new(vyn_relay::proto::PutManifestRequest {
                project_id: project_id.to_string(),
                payload: manifest_payload.to_vec(),
            }))
            .await?;
        client
            .put_manifest(req)
            .await
            .map_err(|err| StorageError::Transport(err.to_string()))?;
        Ok(())
    }

    async fn upload_blob(&self, hash: &str, data: Vec<u8>) -> StorageResult<()> {
        let mut client = self.connect().await?;
        let message = vyn_relay::proto::UploadBlobChunk {
            hash: hash.to_string(),
            chunk: data,
        };

        let mut req = Request::new(tokio_stream::iter(vec![message]));
        if let Some(ref tok) = *self.token.read().await {
            let val = MetadataValue::try_from(tok.as_str())
                .map_err(|e| StorageError::Transport(e.to_string()))?;
            req.metadata_mut().insert("x-vyn-token", val);
        }
        client
            .upload_blob(req)
            .await
            .map_err(|err| StorageError::Transport(err.to_string()))?;
        Ok(())
    }

    async fn download_blob(&self, hash: &str) -> StorageResult<Option<Vec<u8>>> {
        let mut client = self.connect().await?;
        let stream = client
            .download_blob(Request::new(vyn_relay::proto::DownloadBlobRequest {
                hash: hash.to_string(),
            }))
            .await
            .map_err(|err| {
                if err.code() == tonic::Code::NotFound {
                    StorageError::NotFound
                } else {
                    StorageError::Transport(err.to_string())
                }
            });

        if let Err(StorageError::NotFound) = stream {
            return Ok(None);
        }

        let mut stream = stream?.into_inner();

        let mut out = Vec::new();
        while let Some(chunk) = stream
            .message()
            .await
            .map_err(|err| StorageError::Transport(err.to_string()))?
        {
            out.extend_from_slice(&chunk.chunk);
        }

        Ok(Some(out))
    }

    async fn create_invite(
        &self,
        user_id: &str,
        vault_id: &str,
        payload: Vec<u8>,
    ) -> StorageResult<()> {
        let mut client = self.connect().await?;
        let req = self
            .inject_token(Request::new(vyn_relay::proto::CreateInviteRequest {
                user_id: user_id.to_string(),
                vault_id: vault_id.to_string(),
                payload,
            }))
            .await?;
        client
            .create_invite(req)
            .await
            .map_err(|err| StorageError::Transport(err.to_string()))?;
        Ok(())
    }

    async fn get_invites(&self, user_id: &str, vault_id: &str) -> StorageResult<Vec<Vec<u8>>> {
        let mut client = self.connect().await?;
        let response = client
            .get_invites(Request::new(vyn_relay::proto::GetInvitesRequest {
                user_id: user_id.to_string(),
                vault_id: vault_id.to_string(),
            }))
            .await
            .map_err(|err| StorageError::Transport(err.to_string()))?
            .into_inner();
        Ok(response.payloads)
    }
}

struct IdentityConfig {
    github_username: String,
    ssh_private_key: String,
    ssh_public_key: String,
}

fn load_identity(vault_dir: &Path) -> StorageResult<IdentityConfig> {
    let path = vault_dir.join("identity.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| StorageError::Transport(format!("failed to read identity.toml: {e}")))?;
    let github_username = parse_toml_string(&text, "github_username").ok_or_else(|| {
        StorageError::Transport("missing github_username in identity.toml".into())
    })?;
    let ssh_private_key = parse_toml_string(&text, "ssh_private_key").ok_or_else(|| {
        StorageError::Transport("missing ssh_private_key in identity.toml".into())
    })?;
    let ssh_public_key = parse_toml_string(&text, "ssh_public_key")
        .ok_or_else(|| StorageError::Transport("missing ssh_public_key in identity.toml".into()))?;
    Ok(IdentityConfig {
        github_username,
        ssh_private_key,
        ssh_public_key,
    })
}

fn parse_toml_string(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim().strip_prefix('=')?;
            let val = rest.trim().trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

fn sign_nonce_with_ssh_key(nonce: &[u8], private_key: &Path) -> StorageResult<String> {
    let tmp = tempfile::TempDir::new()
        .map_err(|e| StorageError::Transport(format!("failed to create temp dir: {e}")))?;
    let nonce_file = tmp.path().join("nonce");
    std::fs::write(&nonce_file, nonce)
        .map_err(|e| StorageError::Transport(format!("failed to write nonce: {e}")))?;

    let status = Command::new("ssh-keygen")
        .args([
            "-Y",
            "sign",
            "-f",
            private_key.to_str().unwrap_or(""),
            "-n",
            "vyn",
            nonce_file.to_str().unwrap_or(""),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| StorageError::Transport(format!("failed to run ssh-keygen: {e}")))?;

    if !status.success() {
        return Err(StorageError::Transport("ssh-keygen signing failed".into()));
    }

    let sig_file = tmp.path().join("nonce.sig");
    let sig = std::fs::read_to_string(&sig_file)
        .map_err(|e| StorageError::Transport(format!("failed to read signature: {e}")))?;
    Ok(sig)
}

#[cfg(test)]
mod tests {
    use crate::crypto::secret_bytes;
    use crate::manifest::Manifest;
    use crate::storage::{StorageProvider, decrypt_manifest, encrypt_manifest};

    use super::RelayStorageProvider;

    #[tokio::test]
    async fn relay_roundtrip() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test port");
        let port = listener.local_addr().expect("read local addr").port();

        let data_dir = std::env::temp_dir().join(format!("vyn-relay-it-{}", uuid::Uuid::new_v4()));
        let data_dir_string = data_dir.to_string_lossy().to_string();

        let handle = tokio::spawn(async move {
            let _ = vyn_relay::serve_with_listener(listener, data_dir_string).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let provider = RelayStorageProvider::new(format!("http://127.0.0.1:{port}"));
        // Inject the test bypass token so write RPCs pass auth in test builds.
        *provider.token.write().await = Some("test-bypass".to_string());

        let mut uploaded = false;
        for _ in 0..20 {
            if provider
                .upload_blob("blob123", b"hello-relay".to_vec())
                .await
                .is_ok()
            {
                uploaded = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(uploaded, "blob upload should succeed once relay is ready");
        let blob = provider
            .download_blob("blob123")
            .await
            .expect("blob download should succeed")
            .expect("blob should exist");
        assert_eq!(blob, b"hello-relay");

        let key = secret_bytes(vec![3u8; 32]);
        let manifest = Manifest {
            version: 1,
            files: vec![],
        };
        let payload = encrypt_manifest(&manifest, &key).expect("encrypt manifest");
        provider
            .put_manifest("vault-it", &payload)
            .await
            .expect("put manifest should succeed");
        let pulled = provider
            .get_manifest("vault-it")
            .await
            .expect("get manifest should succeed")
            .expect("manifest should exist");
        let restored = decrypt_manifest(&pulled, &key).expect("decrypt manifest");
        assert_eq!(restored.version, 1);

        provider
            .create_invite("alice", "vault-it", b"invite-data".to_vec())
            .await
            .expect("create invite should succeed");
        let invites = provider
            .get_invites("alice", "vault-it")
            .await
            .expect("get invites should succeed");
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0], b"invite-data");

        handle.abort();
        let _ = std::fs::remove_dir_all(data_dir);
    }
}
