# Encryption & Keys

vyn uses two layers of cryptography: **symmetric encryption** for file content, and **asymmetric key wrapping** for sharing.

## Symmetric encryption (AES-256-GCM)

All blobs and manifests are encrypted with AES-256-GCM using the project key (PK).

- Key size: 256 bits, generated with `ring::rand::SecureRandom`
- Nonce: 96-bit random value, unique per encryption operation, stored alongside the ciphertext
- Ciphertext is content-addressed by SHA-256 of the plaintext

The PK is stored in the OS keychain (secure enclave) and never written to disk in plaintext.

## Asymmetric key wrapping (age + SSH)

When you share a vault with a teammate, the PK is wrapped (encrypted) with their SSH public key using [age](https://age-encryption.org) with the `ssh` recipient plugin.

Supported SSH key types: RSA and Ed25519.

The wrapped invite file (`.age`) can only be decrypted by the holder of the matching SSH private key. The relay stores only the ciphertext and cannot read the PK.

## Key summary

| Key | Role | Storage |
|---|---|---|
| **Project Key (PK)** | Symmetric (AES-256-GCM). Encrypts files and manifests. | OS Keychain |
| **SSH Public Key** | Asymmetric. Used to wrap the PK for a teammate. | Public (GitHub) |
| **SSH Private Key** | Asymmetric. Used to unwrap an invite. | `~/.ssh/id_ed25519` |
| **Nonce** | 96-bit random. Ensures identical files produce different ciphertext. | Stored with each blob |

## Security properties

- **Zero-knowledge relay** — the relay never sees the PK or plaintext
- **No shared passwords** — identity is proven via SSH challenge-response
- **Per-operation nonces** — AES-GCM nonce reuse is prevented by using `ring::rand` for each encryption
- **secrecy crate** — sensitive values are zeroized on drop in memory
