# Quick Start

## 1. Initialize a vault

```bash
vyn init my-project
```

This creates a `.vyn/` directory, generates a 256-bit AES project key stored in your OS keychain, and writes an initial manifest.

## 2. Configure storage

Run the interactive config wizard:

```bash
vyn config
```

Or configure non-interactively for CI:

```bash
# In-memory only (no persistence)
vyn config --provider memory --non-interactive

# Self-hosted relay
vyn config --provider relay --relay-url https://relay.example.com --non-interactive
```

> **Note:** Configure storage before running `vyn auth` if you are using relay storage. Auth registers your identity on the relay.

## 3. Authenticate

```bash
vyn auth
```

This runs a 3-step flow:
1. GitHub OAuth Device Flow — opens your browser. Confirm the one-time code.
2. SSH key detection — finds `~/.ssh/id_ed25519` or `~/.ssh/id_rsa` automatically.
3. SSH challenge-response — proves you hold the private key matching your GitHub-registered public key.

On success, writes `.vyn/identity.toml`.

## 4. Push

```bash
vyn push
```

Encrypts tracked files and uploads encrypted blobs + manifest to the configured storage.

## 5. Pull

```bash
vyn pull
```

Downloads the encrypted manifest and blobs, decrypts in memory, and writes plaintext files to disk.

---

## Next steps

- Run `vyn st` to see local changes against the baseline
- Run `vyn diff` to inspect line-level changes
- Share the vault with a teammate: `vyn share @teammate`
- See all commands in the [CLI Reference](../cli/init.md)
