use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use age::Decryptor;
use secrecy::ExposeSecret;
use thiserror::Error;

use crate::crypto::{SecretBytes, secret_bytes};

/// Structured invite payload produced by `unwrap_invite_with_ssh_identity_file`.
pub struct InvitePayload {
    pub vault_id: String,
    pub relay_url: Option<String>,
    pub key: SecretBytes,
}

#[derive(Debug, Error)]
pub enum WrappingError {
    #[error("invalid recipient public key: {0}")]
    InvalidRecipient(String),
    #[error("encryption setup failed")]
    EncryptSetup,
    #[error("failed to write encrypted payload")]
    EncryptWrite,
    #[error("failed to finalize encrypted payload")]
    EncryptFinish,
    #[error("invalid encrypted invite format")]
    InvalidEncryptedInvite,
    #[error("failed to open SSH private key file: {0}")]
    IdentityOpen(#[from] std::io::Error),
    #[error("failed to parse SSH private key identity")]
    IdentityParse,
    #[error("failed to decrypt invite with SSH identity")]
    DecryptFailure,
    #[error("decrypted project key has invalid size")]
    InvalidProjectKeySize,
}

pub fn wrap_project_key_for_ssh_recipient(
    project_key: &SecretBytes,
    recipient_public_key: &str,
) -> Result<Vec<u8>, WrappingError> {
    let recipient: age::ssh::Recipient =
        recipient_public_key
            .trim()
            .parse()
            .map_err(|e: age::ssh::ParseRecipientKeyError| {
                WrappingError::InvalidRecipient(format!("{e:?}"))
            })?;

    let recipients = [recipient];
    let encryptor =
        age::Encryptor::with_recipients(recipients.iter().map(|r| r as &dyn age::Recipient))
            .map_err(|_| WrappingError::EncryptSetup)?;

    let mut output = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut output)
        .map_err(|_| WrappingError::EncryptSetup)?;
    writer
        .write_all(project_key.expose_secret())
        .map_err(|_| WrappingError::EncryptWrite)?;
    writer.finish().map_err(|_| WrappingError::EncryptFinish)?;

    Ok(output)
}

pub fn unwrap_project_key_with_ssh_identity_file(
    encrypted_invite: &[u8],
    identity_file: &Path,
) -> Result<SecretBytes, WrappingError> {
    let decryptor =
        Decryptor::new(encrypted_invite).map_err(|_| WrappingError::InvalidEncryptedInvite)?;

    let file = File::open(identity_file)?;
    let mut reader = BufReader::new(file);
    let identity = age::ssh::Identity::from_buffer(&mut reader, None)
        .map_err(|_| WrappingError::IdentityParse)?;

    let identities = vec![&identity as &dyn age::Identity];
    let mut reader = decryptor
        .decrypt(identities.into_iter())
        .map_err(|_| WrappingError::DecryptFailure)?;

    let mut plaintext = Vec::new();
    reader
        .read_to_end(&mut plaintext)
        .map_err(|_| WrappingError::DecryptFailure)?;

    if plaintext.len() != 32 {
        return Err(WrappingError::InvalidProjectKeySize);
    }

    Ok(secret_bytes(plaintext))
}

/// Wraps a project key into an age-encrypted invite that carries vault metadata.
///
/// The inner plaintext is a JSON object:
/// `{"vault_id":"…","relay_url":"…","key":"<hex>"}` (relay_url omitted when None).
pub fn wrap_invite_for_ssh_recipient(
    project_key: &SecretBytes,
    vault_id: &str,
    relay_url: Option<&str>,
    recipient_public_key: &str,
) -> Result<Vec<u8>, WrappingError> {
    let key_hex = hex::encode(project_key.expose_secret());
    let json = match relay_url {
        Some(url) => {
            format!(r#"{{"vault_id":"{vault_id}","relay_url":"{url}","key":"{key_hex}"}}"#)
        }
        None => format!(r#"{{"vault_id":"{vault_id}","key":"{key_hex}"}}"#),
    };

    let recipient: age::ssh::Recipient =
        recipient_public_key
            .trim()
            .parse()
            .map_err(|e: age::ssh::ParseRecipientKeyError| {
                WrappingError::InvalidRecipient(format!("{e:?}"))
            })?;

    let encryptor =
        age::Encryptor::with_recipients([&recipient as &dyn age::Recipient].into_iter())
            .map_err(|_| WrappingError::EncryptSetup)?;

    let mut output = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut output)
        .map_err(|_| WrappingError::EncryptSetup)?;
    writer
        .write_all(json.as_bytes())
        .map_err(|_| WrappingError::EncryptWrite)?;
    writer.finish().map_err(|_| WrappingError::EncryptFinish)?;

    Ok(output)
}

/// Decrypts an invite created by `wrap_invite_for_ssh_recipient`.
///
/// Supports both the new JSON format and the legacy raw-32-byte format.
pub fn unwrap_invite_with_ssh_identity_file(
    encrypted_invite: &[u8],
    identity_file: &Path,
) -> Result<InvitePayload, WrappingError> {
    let decryptor =
        Decryptor::new(encrypted_invite).map_err(|_| WrappingError::InvalidEncryptedInvite)?;

    let file = File::open(identity_file)?;
    let mut reader = BufReader::new(file);
    let identity = age::ssh::Identity::from_buffer(&mut reader, None)
        .map_err(|_| WrappingError::IdentityParse)?;

    let identities = vec![&identity as &dyn age::Identity];
    let mut decrypted_reader = decryptor
        .decrypt(identities.into_iter())
        .map_err(|_| WrappingError::DecryptFailure)?;

    let mut plaintext = Vec::new();
    decrypted_reader
        .read_to_end(&mut plaintext)
        .map_err(|_| WrappingError::DecryptFailure)?;

    // Try JSON format first.
    if let Ok(text) = std::str::from_utf8(&plaintext)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(text)
        && let Some(key_hex) = v.get("key").and_then(|k| k.as_str())
        && let Ok(key_bytes) = hex::decode(key_hex)
        && key_bytes.len() == 32
    {
        let vault_id = v
            .get("vault_id")
            .and_then(|k| k.as_str())
            .unwrap_or("")
            .to_string();
        let relay_url = v
            .get("relay_url")
            .and_then(|k| k.as_str())
            .map(str::to_string);
        return Ok(InvitePayload {
            vault_id,
            relay_url,
            key: secret_bytes(key_bytes),
        });
    }

    // Legacy format: raw 32-byte key with no embedded metadata.
    if plaintext.len() == 32 {
        return Ok(InvitePayload {
            vault_id: String::new(),
            relay_url: None,
            key: secret_bytes(plaintext),
        });
    }

    Err(WrappingError::InvalidProjectKeySize)
}

#[cfg(test)]
mod tests {
    use super::{unwrap_project_key_with_ssh_identity_file, wrap_project_key_for_ssh_recipient};
    use crate::crypto::secret_bytes;
    use secrecy::ExposeSecret;
    use std::fs;
    use uuid::Uuid;

    const ED25519_PRIVATE: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n\
QyNTUxOQAAACC08MnmfkXbvDUS6ZCCLP+IVNuHmnR6xmfIm3grO/i8eAAAAKC9VdgAvVXY\n\
AAAAAAtzc2gtZWQyNTUxOQAAACC08MnmfkXbvDUS6ZCCLP+IVNuHmnR6xmfIm3grO/i8eA\n\
AAAEAe+MvbyIgxPxS9Q0z17bjL4zmDhgTgal6UxuwRHkGYSLTwyeZ+Rdu8NRLpkIIs/4hU\n\
24eadHrGZ8ibeCs7+Lx4AAAAFmlja2RldkBMQVBUT1AtSThRVUQxN1YBAgMEBQYH\n\
-----END OPENSSH PRIVATE KEY-----\n";

    const ED25519_PUBLIC: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILTwyeZ+Rdu8NRLpkIIs/4hU24eadHrGZ8ibeCs7+Lx4";

    const RSA_PRIVATE: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAABFwAAAAdzc2gtcn\n\
NhAAAAAwEAAQAAAQEA2d2TQ47WLUaiHx3eDA/jAj2OJp9dBMQUnwkZJ9PwzCK4z/oLSgVO\n\
yEdH1sy0XlfCfsDrc0FLjA2quDaC6tp137eI5qQJKIW/nvY+zHTKKdyRAAJ7AoeSq9niDn\n\
lOCmnjsTJpDwDhezL+G7WK4FXsNd2qX6nsd4ZePwd5GjRILQnzwHEAwAb+0u9M+kqk99Nf\n\
btum350ATqBNErxmMHf7qYphG+cwjhqaaJpOx7NPPSRcntw9vM79CXU/uhCTd2OKj6bkuR\n\
IxZZqMqyrQwwaYCfzv5c5ReGvmD/xt8xq9vJbMAYOJg7Pn6ewVm4RxcES3TwkTGqnec+yL\n\
eoJ7ZHHizQAAA9DVhT/p1YU/6QAAAAdzc2gtcnNhAAABAQDZ3ZNDjtYtRqIfHd4MD+MCPY\n\
4mn10ExBSfCRkn0/DMIrjP+gtKBU7IR0fWzLReV8J+wOtzQUuMDaq4NoLq2nXft4jmpAko\n\
hb+e9j7MdMop3JEAAnsCh5Kr2eIOeU4KaeOxMmkPAOF7Mv4btYrgVew13apfqex3hl4/B3\n\
kaNEgtCfPAcQDABv7S70z6SqT3019u26bfnQBOoE0SvGYwd/upimEb5zCOGppomk7Hs089\n\
JFye3D28zv0JdT+6EJN3Y4qPpuS5EjFlmoyrKtDDBpgJ/O/lzlF4a+YP/G3zGr28lswBg4\n\
mDs+fp7BWbhHFwRLdPCRMaqd5z7It6gntkceLNAAAAAwEAAQAAAQAZzu04hg2qHGFtJTke\n\
Ha2rIMabnapDu8SjmEzSEoHGdOCGxpyavqk4AXWppONC/8tq/4iExTnhU+ci3lZA4vMuts\n\
uxYsIw+jMabho/VyBxuA63PRP8VzoRQITOaSFNC4EtBwc5/0U2tnIyrx1N+O+768/YeEUq\n\
XZEBj22RpJreNsUi740JpAGTC66GvGxWYDNv3GLCmTn6f9/kZPzxqyn7f9Q9W4VL4Tz7v5\n\
t5tClFT2JiGKP9V88rzoMCVWxdsbQ32eh00otIX94ll3zROPN8C+zweIe2Dln949gynwX/\n\
xsGF1TUAF0okooJG/5OwIlcqH42utCzM7Xt9rKSZ3BIzAAAAgQCEdYN8bVWtyn8PKyIR5f\n\
PIxzjDymFi6oKeTstfDX4hW3KMlx9Ssw12beT94A9F8HxZ3n94lenmMMWZTRK5GHWlr+Bc\n\
rQDJ0OCMWgNX/9yvIV7CABcJEq1HN0zAw2r3SEKykvavrZYdteNOv6xUgeIG31lMc1415B\n\
GwUdRe5cg36QAAAIEA8BNs3rPmZKiwhgPMchMwvbPy3j3A5ueKszJSfSrmXH9CDaVeeegr\n\
RRXQ+bvg08ddnylez1sXmi8BnpNEBbLloF77mnJfydyjw680+GmLhApSh9cL/DQRhb3fVZ\n\
S5s8lkQC2NZz2Gng+kopUJHjoGT2K6DjGorgqcXtA60VNsGNcAAACBAOhRAp313i8PpICM\n\
P/Cez1Gdoi4+o5Pt/Tx7qcE7gxdizr1wHKIoWWKDhS6vq2hvPgDtEoYj/n7xrSj/FUXtk3\n\
/tTCGT39tUKpZ8ECVBoZHDxRVkqNZo+DExGELwtskxrSL/g3xmpcVQF9Yp83I3vkGGbP/s\n\
KI+W/Y8KVMd63bj7AAAAFmlja2RldkBMQVBUT1AtSThRVUQxN1YBAgME\n\
-----END OPENSSH PRIVATE KEY-----\n";

    const RSA_PUBLIC: &str = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQDZ3ZNDjtYtRqIfHd4MD+MCPY4mn10ExBSfCRkn0/DMIrjP+gtKBU7IR0fWzLReV8J+wOtzQUuMDaq4NoLq2nXft4jmpAkohb+e9j7MdMop3JEAAnsCh5Kr2eIOeU4KaeOxMmkPAOF7Mv4btYrgVew13apfqex3hl4/B3kaNEgtCfPAcQDABv7S70z6SqT3019u26bfnQBOoE0SvGYwd/upimEb5zCOGppomk7Hs089JFye3D28zv0JdT+6EJN3Y4qPpuS5EjFlmoyrKtDDBpgJ/O/lzlF4a+YP/G3zGr28lswBg4mDs+fp7BWbhHFwRLdPCRMaqd5z7It6gntkceLN";

    #[test]
    fn ssh_key_wrapping_ed25519() {
        run_roundtrip(ED25519_PRIVATE, ED25519_PUBLIC);
    }

    #[test]
    fn ssh_key_wrapping_rsa() {
        run_roundtrip(RSA_PRIVATE, RSA_PUBLIC);
    }

    fn run_roundtrip(private: &str, public: &str) {
        let tmp = std::env::temp_dir().join(format!("vyn-wrap-fixture-{}", Uuid::new_v4()));
        fs::create_dir_all(&tmp).expect("temp directory should be created");
        let private_key = tmp.join("id_key");
        fs::write(&private_key, private).expect("private key fixture should be written");
        let key = secret_bytes(vec![11u8; 32]);

        let encrypted =
            wrap_project_key_for_ssh_recipient(&key, public).expect("wrapping should succeed");
        let unwrapped = unwrap_project_key_with_ssh_identity_file(&encrypted, &private_key)
            .expect("unwrapping should succeed");

        assert_eq!(key.expose_secret(), unwrapped.expose_secret());

        fs::remove_dir_all(tmp).expect("temp directory should be removed");
    }
}
