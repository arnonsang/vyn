use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use ring::rand::SecureRandom;

use crate::auth::{generate_nonce, verify_registration_signature, verify_ssh_signature};
use crate::proto::vyn_relay_server::VynRelay;
use crate::proto::*;
use crate::store::{FileStore, sanitize_id};

const CHALLENGE_TTL: Duration = Duration::from_secs(60);
const SESSION_TTL: Duration = Duration::from_secs(86400);

type ChallengeMap = Arc<RwLock<HashMap<String, (Vec<u8>, Instant)>>>;

#[derive(Clone)]
pub struct RelayService {
    store: FileStore,
    challenges: ChallengeMap,
    sessions: Arc<RwLock<HashMap<String, Instant>>>,
}

impl RelayService {
    pub fn new(store: FileStore) -> Self {
        Self {
            store,
            challenges: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[allow(clippy::result_large_err)]
fn require_auth<T>(
    request: &Request<T>,
    sessions: &HashMap<String, Instant>,
) -> Result<(), Status> {
    // When the test-bypass-auth feature is enabled, allow a magic token so
    // integration tests can exercise storage without a real SSH key setup.
    #[cfg(feature = "test-bypass-auth")]
    if request
        .metadata()
        .get("x-vyn-token")
        .and_then(|v| v.to_str().ok())
        == Some("test-bypass")
    {
        return Ok(());
    }

    let token = request
        .metadata()
        .get("x-vyn-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Status::unauthenticated("missing x-vyn-token header"))?;
    match sessions.get(token) {
        Some(issued_at) if issued_at.elapsed() < SESSION_TTL => Ok(()),
        Some(_) | None => Err(Status::unauthenticated("invalid or expired token")),
    }
}

type DownloadBlobStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<DownloadBlobChunk, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl VynRelay for RelayService {
    async fn authenticate(
        &self,
        request: Request<AuthRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let req = request.into_inner();
        let user_id =
            sanitize_id(&req.user_id).map_err(|e| Status::invalid_argument(e.to_string()))?;

        if req.signature.trim().is_empty() {
            let nonce = generate_nonce().map_err(|e| Status::internal(e.to_string()))?;
            let mut map = self.challenges.write().await;
            let now = Instant::now();
            map.retain(|_, (_, issued_at)| now.duration_since(*issued_at) < CHALLENGE_TTL);
            map.insert(user_id, (nonce.clone(), now));
            return Ok(Response::new(AuthResponse {
                authenticated: false,
                challenge_nonce: nonce,
                message: "challenge_issued".to_string(),
                token: String::new(),
            }));
        }

        let (expected, issued_at) = self
            .challenges
            .write()
            .await
            .remove(&user_id)
            .ok_or_else(|| Status::failed_precondition("no active challenge for user"))?;

        if Instant::now().duration_since(issued_at) >= CHALLENGE_TTL {
            return Err(Status::deadline_exceeded("challenge expired"));
        }

        if req.nonce != expected {
            return Err(Status::unauthenticated("invalid challenge nonce"));
        }

        let public_key = self
            .store
            .get_identity(&user_id)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("identity not registered"))?;

        let ok = verify_ssh_signature(&user_id, &public_key, &req.nonce, &req.signature)
            .map_err(|e| Status::internal(e.to_string()))?;

        let token = if ok {
            let mut raw = vec![0u8; 32];
            ring::rand::SystemRandom::new()
                .fill(&mut raw)
                .map_err(|_| Status::internal("failed to generate session token"))?;
            let tok: String = raw.iter().map(|b| format!("{b:02x}")).collect();
            self.sessions
                .write()
                .await
                .insert(tok.clone(), Instant::now());
            tok
        } else {
            String::new()
        };

        Ok(Response::new(AuthResponse {
            authenticated: ok,
            challenge_nonce: Vec::new(),
            message: if ok {
                "authenticated"
            } else {
                "invalid_signature"
            }
            .to_string(),
            token,
        }))
    }

    async fn register_identity(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let user_id =
            sanitize_id(&req.user_id).map_err(|e| Status::invalid_argument(e.to_string()))?;
        let public_key = req.public_key.trim().to_string();
        if public_key.is_empty() {
            return Err(Status::invalid_argument("public_key is required"));
        }
        if req.signature.trim().is_empty() {
            return Err(Status::unauthenticated(
                "signature is required to prove key ownership",
            ));
        }

        let ok = verify_registration_signature(&user_id, &public_key, req.signature.trim())
            .map_err(|e| Status::internal(e.to_string()))?;

        if !ok {
            return Err(Status::permission_denied(
                "proof-of-possession signature verification failed",
            ));
        }

        self.store
            .put_identity(&user_id, &public_key)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RegisterResponse {
            ok: true,
            message: "registered".to_string(),
        }))
    }

    async fn get_manifest(
        &self,
        request: Request<GetManifestRequest>,
    ) -> Result<Response<GetManifestResponse>, Status> {
        let project_id = sanitize_id(&request.into_inner().project_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let payload = self
            .store
            .get_manifest(&project_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        match payload {
            Some(payload) => Ok(Response::new(GetManifestResponse {
                payload,
                found: true,
            })),
            None => Ok(Response::new(GetManifestResponse {
                payload: Vec::new(),
                found: false,
            })),
        }
    }

    async fn put_manifest(
        &self,
        request: Request<PutManifestRequest>,
    ) -> Result<Response<PutManifestResponse>, Status> {
        require_auth(&request, &*self.sessions.read().await)?;
        let req = request.into_inner();
        let project_id =
            sanitize_id(&req.project_id).map_err(|e| Status::invalid_argument(e.to_string()))?;
        self.store
            .put_manifest(&project_id, &req.payload)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(PutManifestResponse { ok: true }))
    }

    async fn upload_blob(
        &self,
        request: Request<tonic::Streaming<UploadBlobChunk>>,
    ) -> Result<Response<UploadBlobResponse>, Status> {
        {
            let sessions = self.sessions.read().await;
            require_auth(&request, &sessions)?;
        }
        let mut stream = request.into_inner();
        let mut hash: Option<String> = None;
        let mut payload = Vec::new();

        while let Some(chunk) = stream.message().await? {
            let chunk_hash =
                sanitize_id(&chunk.hash).map_err(|e| Status::invalid_argument(e.to_string()))?;
            if hash.is_none() {
                hash = Some(chunk_hash);
            } else if !chunk.hash.is_empty() && Some(&chunk_hash) != hash.as_ref() {
                return Err(Status::invalid_argument("chunk hash mismatch"));
            }
            payload.extend_from_slice(&chunk.chunk);
        }

        let hash = hash.ok_or_else(|| Status::invalid_argument("upload stream had no chunks"))?;
        self.store
            .put_blob(&hash, &payload)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(UploadBlobResponse { ok: true }))
    }

    type DownloadBlobStream = DownloadBlobStream;

    async fn download_blob(
        &self,
        request: Request<DownloadBlobRequest>,
    ) -> Result<Response<Self::DownloadBlobStream>, Status> {
        let hash = sanitize_id(&request.into_inner().hash)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let payload = self
            .store
            .get_blob(&hash)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("blob not found"))?;

        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            const CHUNK: usize = 64 * 1024;
            let mut offset = 0usize;
            while offset < payload.len() {
                let end = (offset + CHUNK).min(payload.len());
                let message = DownloadBlobChunk {
                    chunk: payload[offset..end].to_vec(),
                };
                if tx.send(Ok(message)).await.is_err() {
                    return;
                }
                offset = end;
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn create_invite(
        &self,
        request: Request<CreateInviteRequest>,
    ) -> Result<Response<CreateInviteResponse>, Status> {
        require_auth(&request, &*self.sessions.read().await)?;
        let req = request.into_inner();
        let user_id =
            sanitize_id(&req.user_id).map_err(|e| Status::invalid_argument(e.to_string()))?;
        let vault_id =
            sanitize_id(&req.vault_id).map_err(|e| Status::invalid_argument(e.to_string()))?;

        self.store
            .put_invite(&user_id, &vault_id, &req.payload)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateInviteResponse { ok: true }))
    }

    async fn get_invites(
        &self,
        request: Request<GetInvitesRequest>,
    ) -> Result<Response<GetInvitesResponse>, Status> {
        let req = request.into_inner();
        let user_id =
            sanitize_id(&req.user_id).map_err(|e| Status::invalid_argument(e.to_string()))?;
        let vault_id =
            sanitize_id(&req.vault_id).map_err(|e| Status::invalid_argument(e.to_string()))?;

        let payloads = self
            .store
            .get_invites(&user_id, &vault_id)
            .await
            .context("failed to read invites")
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetInvitesResponse { payloads }))
    }

    async fn list_vaults(
        &self,
        request: Request<ListVaultsRequest>,
    ) -> Result<Response<ListVaultsResponse>, Status> {
        require_auth(&request, &*self.sessions.read().await)?;

        let vault_ids = self
            .store
            .list_vaults()
            .context("failed to list vaults")
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ListVaultsResponse { vault_ids }))
    }

    async fn list_blobs(
        &self,
        request: Request<ListBlobsRequest>,
    ) -> Result<Response<ListBlobsResponse>, Status> {
        require_auth(&request, &*self.sessions.read().await)?;

        let blobs = self
            .store
            .list_blobs()
            .context("failed to list blobs")
            .map_err(|e| Status::internal(e.to_string()))?
            .into_iter()
            .map(|(sha256, size_bytes)| BlobInfo { sha256, size_bytes })
            .collect();

        Ok(Response::new(ListBlobsResponse { blobs }))
    }
}
