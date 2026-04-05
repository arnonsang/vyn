use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use ring::rand::{SecureRandom, SystemRandom};

pub fn generate_nonce() -> Result<Vec<u8>> {
    let mut nonce = vec![0u8; 32];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow::anyhow!("failed to generate secure nonce"))?;
    Ok(nonce)
}

/// Verify a challenge-response SSH signature (nonce is relay-issued bytes).
pub fn verify_ssh_signature(
    user_id: &str,
    public_key: &str,
    nonce: &[u8],
    signature: &str,
) -> Result<bool> {
    let temp = tempfile::TempDir::new().context("failed to create auth temp dir")?;

    let allowed_signers = temp.path().join("allowed_signers");
    let signature_file = temp.path().join("signature");

    fs::write(&allowed_signers, format!("{} {}\n", user_id, public_key))
        .context("failed to write allowed signers")?;
    fs::write(&signature_file, signature.as_bytes()).context("failed to write signature file")?;

    run_ssh_verify(user_id, &allowed_signers, &signature_file, nonce)
}

/// Verify a registration proof-of-possession: client must sign
/// `"vyn-register:{user_id}:{public_key}"` with the private key that matches
/// `public_key`.  This proves ownership before we store the identity on the relay.
pub fn verify_registration_signature(
    user_id: &str,
    public_key: &str,
    signature: &str,
) -> Result<bool> {
    let payload = format!("vyn-register:{user_id}:{public_key}");
    verify_ssh_signature(user_id, public_key, payload.as_bytes(), signature)
}

fn run_ssh_verify(
    user_id: &str,
    allowed_signers: &Path,
    signature_file: &Path,
    nonce: &[u8],
) -> Result<bool> {
    let mut cmd = Command::new("ssh-keygen");
    cmd.arg("-Y")
        .arg("verify")
        .arg("-f")
        .arg(allowed_signers)
        .arg("-I")
        .arg(user_id)
        .arg("-n")
        .arg("vyn")
        .arg("-s")
        .arg(signature_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd
        .spawn()
        .context("failed to spawn ssh-keygen for signature verification")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(nonce)
            .context("failed to write nonce to verifier")?;
    }

    let status = child.wait().context("failed waiting for verifier")?;
    Ok(status.success())
}
