use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use aws_sdk_s3::error::SdkError;

use crate::ServeConfig;

#[derive(Clone)]
pub struct FileStore {
    root: PathBuf,
    s3: Option<S3Mirror>,
}

#[derive(Clone)]
struct S3Mirror {
    client: Client,
    bucket: String,
    prefix: String,
}

impl FileStore {
    pub async fn new(root: impl Into<PathBuf>, config: ServeConfig) -> Result<Self> {
        let s3 = if let (Some(bucket), Some(region)) = (config.s3_bucket, config.s3_region) {
            let shared = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(region))
                .load()
                .await;
            let mut builder = S3ConfigBuilder::from(&shared);
            if let Some(endpoint) = config.s3_endpoint {
                builder = builder.endpoint_url(endpoint);
            }
            let client = Client::from_conf(builder.build());
            let prefix = config.s3_prefix.unwrap_or_else(|| "vyn-relay".to_string());
            Some(S3Mirror {
                client,
                bucket,
                prefix,
            })
        } else {
            None
        };

        Ok(Self {
            root: root.into(),
            s3,
        })
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.manifests_dir()).context("failed to create manifests directory")?;
        fs::create_dir_all(self.blobs_dir()).context("failed to create blobs directory")?;
        fs::create_dir_all(self.invites_dir()).context("failed to create invites directory")?;
        fs::create_dir_all(self.identities_dir())
            .context("failed to create identities directory")?;
        Ok(())
    }

    pub async fn put_manifest(&self, project_id: &str, payload: &[u8]) -> Result<()> {
        fs::write(self.manifest_path(project_id), payload).context("failed to write manifest")?;
        if let Some(s3) = &self.s3
            && let Err(err) = s3.put_object(&s3.key_manifest(project_id), payload).await
        {
            eprintln!("relay warning: failed to mirror manifest to S3: {err}");
        }
        Ok(())
    }

    pub async fn get_manifest(&self, project_id: &str) -> Result<Option<Vec<u8>>> {
        let path = self.manifest_path(project_id);
        if !path.exists() {
            if let Some(s3) = &self.s3 {
                let payload = s3.get_object(&s3.key_manifest(project_id)).await?;
                if let Some(ref data) = payload {
                    let _ = fs::write(&path, data);
                }
                return Ok(payload);
            }
            return Ok(None);
        }
        Ok(Some(fs::read(path).context("failed to read manifest")?))
    }

    pub async fn put_blob(&self, hash: &str, payload: &[u8]) -> Result<()> {
        fs::write(self.blob_path(hash), payload).context("failed to write blob")?;
        if let Some(s3) = &self.s3
            && let Err(err) = s3.put_object(&s3.key_blob(hash), payload).await
        {
            eprintln!("relay warning: failed to mirror blob to S3: {err}");
        }
        Ok(())
    }

    pub async fn get_blob(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        let path = self.blob_path(hash);
        if !path.exists() {
            if let Some(s3) = &self.s3 {
                let payload = s3.get_object(&s3.key_blob(hash)).await?;
                if let Some(ref data) = payload {
                    let _ = fs::write(&path, data);
                }
                return Ok(payload);
            }
            return Ok(None);
        }
        Ok(Some(fs::read(path).context("failed to read blob")?))
    }

    pub async fn put_invite(&self, user_id: &str, vault_id: &str, payload: &[u8]) -> Result<()> {
        let dir = self.invite_vault_dir(user_id, vault_id);
        fs::create_dir_all(&dir).context("failed to create invite vault directory")?;
        let file_name = format!("{}.age", uuid::Uuid::new_v4());
        fs::write(dir.join(&file_name), payload).context("failed to write invite")?;

        if let Some(s3) = &self.s3 {
            let key = s3.key_invite(user_id, vault_id, &file_name);
            if let Err(err) = s3.put_object(&key, payload).await {
                eprintln!("relay warning: failed to mirror invite to S3: {err}");
            }
        }

        Ok(())
    }

    pub async fn get_invites(&self, user_id: &str, vault_id: &str) -> Result<Vec<Vec<u8>>> {
        let dir = self.invite_vault_dir(user_id, vault_id);
        if !dir.exists() {
            if let Some(s3) = &self.s3 {
                return s3.get_invites(user_id, vault_id).await;
            }
            return Ok(Vec::new());
        }

        let mut out = Vec::new();
        for entry in fs::read_dir(&dir).context("failed to list invites")? {
            let path = entry.context("failed to read invite entry")?.path();
            if path.is_file() {
                out.push(fs::read(path).context("failed to read invite payload")?);
            }
        }
        Ok(out)
    }

    pub fn put_identity(&self, user_id: &str, public_key: &str) -> Result<()> {
        fs::write(self.identity_path(user_id), public_key.as_bytes())
            .context("failed to write identity")
    }

    pub fn get_identity(&self, user_id: &str) -> Result<Option<String>> {
        let path = self.identity_path(user_id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(
            fs::read_to_string(path)
                .context("failed to read identity")?
                .trim()
                .to_string(),
        ))
    }

    fn manifests_dir(&self) -> PathBuf {
        self.root.join("manifests")
    }

    fn blobs_dir(&self) -> PathBuf {
        self.root.join("blobs")
    }

    fn invites_dir(&self) -> PathBuf {
        self.root.join("invites")
    }

    fn identities_dir(&self) -> PathBuf {
        self.root.join("identities")
    }

    fn manifest_path(&self, project_id: &str) -> PathBuf {
        self.manifests_dir().join(format!("{project_id}.enc"))
    }

    fn blob_path(&self, hash: &str) -> PathBuf {
        self.blobs_dir().join(format!("{hash}.enc"))
    }

    fn invite_vault_dir(&self, user_id: &str, vault_id: &str) -> PathBuf {
        self.invites_dir().join(user_id).join(vault_id)
    }

    fn identity_path(&self, user_id: &str) -> PathBuf {
        self.identities_dir().join(format!("{user_id}.pub"))
    }
}

impl S3Mirror {
    fn key_manifest(&self, project_id: &str) -> String {
        format!("{}/manifests/{project_id}.enc", self.prefix)
    }

    fn key_blob(&self, hash: &str) -> String {
        format!("{}/blobs/{hash}.enc", self.prefix)
    }

    fn key_invite(&self, user_id: &str, vault_id: &str, file_name: &str) -> String {
        format!("{}/invites/{user_id}/{vault_id}/{file_name}", self.prefix)
    }

    async fn put_object(&self, key: &str, payload: &[u8]) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(payload.to_vec().into())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match response {
            Ok(output) => {
                let bytes = output
                    .body
                    .collect()
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .into_bytes();
                Ok(Some(bytes.to_vec()))
            }
            Err(SdkError::ServiceError(err)) if err.err().is_no_such_key() => Ok(None),
            Err(err) => Err(anyhow::anyhow!(err.to_string())),
        }
    }

    async fn get_invites(&self, user_id: &str, vault_id: &str) -> Result<Vec<Vec<u8>>> {
        let prefix = format!("{}/invites/{}/{}/", self.prefix, user_id, vault_id);
        let listed = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let mut out = Vec::new();
        for object in listed.contents() {
            if let Some(key) = object.key()
                && let Some(payload) = self.get_object(key).await?
            {
                out.push(payload);
            }
        }

        Ok(out)
    }
}

pub fn sanitize_id(input: &str) -> Result<String> {
    if input.is_empty() {
        anyhow::bail!("identifier cannot be empty");
    }
    if input.len() > 128 {
        anyhow::bail!("identifier too long");
    }
    if !input
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        anyhow::bail!("identifier contains invalid characters (allowed: a-z A-Z 0-9 - _ .)");
    }
    Ok(input.to_string())
}

pub fn ensure_within(base: &Path, candidate: &Path) -> Result<()> {
    let base = base
        .canonicalize()
        .context("failed to canonicalize base path")?;
    let candidate = candidate
        .canonicalize()
        .context("failed to canonicalize candidate path")?;
    if !candidate.starts_with(&base) {
        anyhow::bail!("path traversal detected");
    }
    Ok(())
}
