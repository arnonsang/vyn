use keyring::Entry;
use secrecy::ExposeSecret;
use thiserror::Error;

use crate::crypto::SecretBytes;

const SERVICE_NAME: &str = "vyn";
const PROJECT_KEY_LEN: usize = 32;

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("project key must be 32 bytes")]
    InvalidKeyLength,
    #[error("failed to encode key")]
    EncodingFailure,
    #[error("failed to decode key")]
    DecodingFailure,
    #[error("keychain operation failed: {0}")]
    Keychain(#[from] keyring::Error),
}

pub fn account_for_vault(vault_id: &str) -> String {
    format!("vault_{vault_id}")
}

pub fn store_project_key(vault_id: &str, key: &SecretBytes) -> Result<(), KeychainError> {
    if key.expose_secret().len() != PROJECT_KEY_LEN {
        return Err(KeychainError::InvalidKeyLength);
    }

    let account = account_for_vault(vault_id);
    let entry = Entry::new(SERVICE_NAME, &account)?;
    let encoded = hex_encode(key.expose_secret())?;
    entry.set_password(&encoded)?;

    Ok(())
}

pub fn load_project_key(vault_id: &str) -> Result<SecretBytes, KeychainError> {
    let account = account_for_vault(vault_id);
    let entry = Entry::new(SERVICE_NAME, &account)?;
    let encoded = entry.get_password()?;
    let decoded = hex_decode(&encoded)?;

    if decoded.len() != PROJECT_KEY_LEN {
        return Err(KeychainError::InvalidKeyLength);
    }

    Ok(SecretBytes::new(decoded.into_boxed_slice()))
}

#[cfg(test)]
pub fn delete_project_key(vault_id: &str) -> Result<(), KeychainError> {
    let account = account_for_vault(vault_id);
    let entry = Entry::new(SERVICE_NAME, &account)?;

    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(KeychainError::Keychain(err)),
    }
}

fn hex_encode(bytes: &[u8]) -> Result<String, KeychainError> {
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        use core::fmt::Write;
        write!(&mut output, "{byte:02x}").map_err(|_| KeychainError::EncodingFailure)?;
    }

    Ok(output)
}

fn hex_decode(input: &str) -> Result<Vec<u8>, KeychainError> {
    if !input.len().is_multiple_of(2) {
        return Err(KeychainError::DecodingFailure);
    }

    let mut output = Vec::with_capacity(input.len() / 2);
    let mut idx = 0;
    while idx < input.len() {
        let hi = hex_nibble(input.as_bytes()[idx]).ok_or(KeychainError::DecodingFailure)?;
        let lo = hex_nibble(input.as_bytes()[idx + 1]).ok_or(KeychainError::DecodingFailure)?;
        output.push((hi << 4) | lo);
        idx += 2;
    }

    Ok(output)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{delete_project_key, load_project_key, store_project_key};
    use keyring::credential::{
        Credential, CredentialApi, CredentialBuilderApi, CredentialPersistence,
    };
    use keyring::{Error as KeyringError, set_default_credential_builder};
    use secrecy::{ExposeSecret, SecretBox};
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::Once;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    static INIT_MOCK_KEYRING: Once = Once::new();

    fn ensure_mock_keyring() {
        INIT_MOCK_KEYRING.call_once(|| {
            let shared = Arc::new(Mutex::new(HashMap::<String, Vec<u8>>::new()));
            set_default_credential_builder(Box::new(PersistentMockBuilder { shared }));
        });
    }

    #[derive(Debug)]
    struct PersistentMockBuilder {
        shared: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    impl CredentialBuilderApi for PersistentMockBuilder {
        fn build(
            &self,
            target: Option<&str>,
            service: &str,
            user: &str,
        ) -> keyring::Result<Box<Credential>> {
            let key = format!("{}::{service}::{user}", target.unwrap_or_default());
            Ok(Box::new(PersistentMockCredential {
                shared: Arc::clone(&self.shared),
                key,
            }))
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn persistence(&self) -> CredentialPersistence {
            CredentialPersistence::ProcessOnly
        }
    }

    #[derive(Debug)]
    struct PersistentMockCredential {
        shared: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        key: String,
    }

    impl CredentialApi for PersistentMockCredential {
        fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
            let mut guard = self.shared.lock().map_err(|_| {
                KeyringError::PlatformFailure("mock store poisoned".to_string().into())
            })?;
            guard.insert(self.key.clone(), secret.to_vec());
            Ok(())
        }

        fn get_secret(&self) -> keyring::Result<Vec<u8>> {
            let guard = self.shared.lock().map_err(|_| {
                KeyringError::PlatformFailure("mock store poisoned".to_string().into())
            })?;
            guard.get(&self.key).cloned().ok_or(KeyringError::NoEntry)
        }

        fn delete_credential(&self) -> keyring::Result<()> {
            let mut guard = self.shared.lock().map_err(|_| {
                KeyringError::PlatformFailure("mock store poisoned".to_string().into())
            })?;
            match guard.remove(&self.key) {
                Some(_) => Ok(()),
                None => Err(KeyringError::NoEntry),
            }
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn keychain_persistence() {
        ensure_mock_keyring();

        let vault_id = Uuid::new_v4().to_string();
        let key = SecretBox::new(vec![7u8; 32].into_boxed_slice());

        store_project_key(&vault_id, &key).expect("store should succeed");
        let loaded = load_project_key(&vault_id).expect("load should succeed");
        assert_eq!(loaded.expose_secret(), key.expose_secret());

        delete_project_key(&vault_id).expect("cleanup should succeed");
    }
}
