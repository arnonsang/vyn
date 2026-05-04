# Security Notes

## Encryption

- All blobs and manifests are encrypted with **AES-256-GCM** before leaving the local machine
- The encryption key (project key) is a 256-bit random key stored in the OS keychain
- The relay and S3 backend never see plaintext content or metadata (zero-knowledge)

## Key Storage

Project keys are stored in the OS keychain:

| Platform | Backend |
|---|---|
| Linux | `keyutils` (kernel keyring) or Secret Service (D-Bus) |
| macOS | macOS Keychain |
| Windows | Windows Credential Manager (DPAPI) |

Keys are never written to disk in plaintext.

## Identity + SSH Challenge-Response

`vyn auth` uses GitHub OAuth Device Flow to establish identity, then proves key ownership via a local `ssh-keygen` sign/verify round-trip. No passwords. Your private key never leaves your machine.

## Invite Encryption

Invites (created by `vyn share @user`) are encrypted with the recipient's SSH public key via [age](https://age-encryption.org). Each invite embeds the vault ID, relay URL, and project key - all encrypted specifically for the named recipient.

## Session Tokens

After `vyn auth`, the relay issues a 32-byte cryptographically random session token (hex-encoded). This token is:

- Cached at `.vyn/session.token` with `0600` permissions (owner-read-only on Unix)
- Enforced with a 24-hour TTL server-side - expired tokens require re-auth
- Equivalent in sensitivity to an SSH agent socket; keep `.vyn/` out of Git (handled automatically by `vyn init`)

## Git Safety

`vyn init` adds `.vyn/` to `.gitignore` automatically, preventing accidental commit of:

- `.vyn/config.toml` (contains relay URL and vault ID)
- `.vyn/identity.toml` (SSH key paths)
- `.vyn/session.token` (relay session credential)
- `.vyn/blobs/` (encrypted blob cache)
- `.vyn/manifest.json` (plaintext file list)

`vyn.toml` (vault ID + relay URL, no secrets) is safe to commit and is intended to be committed.

## Relay TLS

The relay does not terminate TLS itself. Use a reverse proxy (nginx, Caddy) for HTTPS. Sending session tokens over plaintext gRPC exposes them to network sniffing - TLS is strongly recommended in production. See [Docker Deployment](../relay/docker.md) and [Relay Overview](../relay/overview.md) for TLS setup.

## Known Limitations

- No way to revoke a teammate's access without full key rotation (`vyn rotate`)
- Invite files accumulate on the relay; no expiry mechanism yet
- Session tokens are valid for 24h - a leaked `.vyn/session.token` grants relay access until TTL expires or relay restarts
- `VYN_SKIP_GITHUB_VERIFY=1` disables SSH key verification; only active in debug builds
