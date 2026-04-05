use crate::crypto::{EncryptedData, SecretBytes, encrypt, secret_bytes};
use ring::aead::NONCE_LEN;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlobError {
    #[error("failed to read source file: {0}")]
    ReadSource(#[from] std::io::Error),
    #[error("failed to encrypt file contents: {0}")]
    Encrypt(#[from] crate::crypto::CryptoError),
    #[error("invalid encrypted blob format")]
    InvalidBlobFormat,
    #[error("failed to decrypt blob: {0}")]
    Decrypt(#[source] crate::crypto::CryptoError),
}

pub fn encrypt_file_to_blob(
    source_file: &Path,
    blobs_dir: &Path,
    key: &SecretBytes,
) -> Result<String, BlobError> {
    let content = fs::read(source_file)?;
    let hash = sha256_hex(&content);
    let encrypted = encrypt(key, &secret_bytes(content))?;

    fs::create_dir_all(blobs_dir)?;
    let blob_path = blob_path(blobs_dir, &hash);
    let payload = encode_blob(&encrypted);
    fs::write(blob_path, payload)?;

    Ok(hash)
}

pub fn blob_path(blobs_dir: &Path, hash: &str) -> PathBuf {
    blobs_dir.join(format!("{hash}.enc"))
}

pub fn decode_blob(payload: &[u8]) -> Result<EncryptedData, BlobError> {
    if payload.len() < NONCE_LEN {
        return Err(BlobError::InvalidBlobFormat);
    }

    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&payload[..NONCE_LEN]);

    Ok(EncryptedData {
        nonce,
        ciphertext: payload[NONCE_LEN..].to_vec(),
    })
}

pub fn decrypt_blob_bytes(blob_file: &Path, key: &SecretBytes) -> Result<SecretBytes, BlobError> {
    let payload = fs::read(blob_file)?;
    let encrypted = decode_blob(&payload)?;
    crate::crypto::decrypt(key, &encrypted).map_err(BlobError::Decrypt)
}

pub fn decrypt_blob_by_hash(
    blobs_dir: &Path,
    hash: &str,
    key: &SecretBytes,
) -> Result<SecretBytes, BlobError> {
    let path = blob_path(blobs_dir, hash);
    decrypt_blob_bytes(&path, key)
}

fn encode_blob(data: &EncryptedData) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.nonce.len() + data.ciphertext.len());
    out.extend_from_slice(&data.nonce);
    out.extend_from_slice(&data.ciphertext);
    out
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
