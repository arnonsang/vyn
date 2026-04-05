use ring::aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use secrecy::{ExposeSecret, SecretBox};
use thiserror::Error;

const PROJECT_KEY_LEN: usize = 32;

pub type SecretBytes = SecretBox<[u8]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedData {
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("project key must be 32 bytes")]
    InvalidKeyLength,
    #[error("failed to generate secure random bytes")]
    RandomFailure,
    #[error("failed to initialize AES-256-GCM key")]
    KeyInitFailure,
    #[error("encryption failed")]
    EncryptionFailure,
    #[error("decryption failed")]
    DecryptionFailure,
}

pub fn secret_bytes(bytes: Vec<u8>) -> SecretBytes {
    SecretBox::new(bytes.into_boxed_slice())
}

pub fn generate_project_key() -> Result<SecretBytes, CryptoError> {
    let mut key = [0u8; PROJECT_KEY_LEN];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| CryptoError::RandomFailure)?;
    Ok(secret_bytes(key.to_vec()))
}

pub fn encrypt(key: &SecretBytes, plaintext: &SecretBytes) -> Result<EncryptedData, CryptoError> {
    let aead_key = build_key(key.expose_secret())?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce_bytes)
        .map_err(|_| CryptoError::RandomFailure)?;

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut buffer = plaintext.expose_secret().to_vec();

    aead_key
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut buffer)
        .map_err(|_| CryptoError::EncryptionFailure)?;

    Ok(EncryptedData {
        nonce: nonce_bytes,
        ciphertext: buffer,
    })
}

pub fn decrypt(key: &SecretBytes, encrypted: &EncryptedData) -> Result<SecretBytes, CryptoError> {
    let aead_key = build_key(key.expose_secret())?;
    let nonce = Nonce::assume_unique_for_key(encrypted.nonce);

    let mut buffer = encrypted.ciphertext.clone();
    let plaintext = aead_key
        .open_in_place(nonce, Aad::empty(), &mut buffer)
        .map_err(|_| CryptoError::DecryptionFailure)?;

    Ok(secret_bytes(plaintext.to_vec()))
}

fn build_key(key: &[u8]) -> Result<LessSafeKey, CryptoError> {
    if key.len() != PROJECT_KEY_LEN {
        return Err(CryptoError::InvalidKeyLength);
    }

    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::KeyInitFailure)?;
    Ok(LessSafeKey::new(unbound))
}

#[cfg(test)]
mod tests {
    use super::{decrypt, encrypt, generate_project_key, secret_bytes};
    use ring::rand::{SecureRandom, SystemRandom};
    use secrecy::ExposeSecret;

    #[test]
    fn aes_gcm_roundtrip() {
        let key = generate_project_key().expect("project key generation should succeed");

        let mut plaintext = vec![0u8; 1024 * 1024];
        SystemRandom::new()
            .fill(&mut plaintext)
            .expect("random data generation should succeed");

        let encrypted =
            encrypt(&key, &secret_bytes(plaintext.clone())).expect("encryption should succeed");
        let decrypted = decrypt(&key, &encrypted).expect("decryption should succeed");

        assert_eq!(decrypted.expose_secret(), plaintext.as_slice());
    }
}
