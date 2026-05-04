# Utility Commands

## vyn run

Run a subprocess with env vars injected from `.env` files and encrypted vault blobs.

```bash
vyn run <cmd...>
```

**What it does:**

1. Merges keys from `.env` and `.env.*` files in the current directory
2. Decrypts and merges `.env` blobs from the vault baseline (when a project key is available)
3. Passes the merged map to the child process environment
4. Does **not** write decrypted values to disk

**Example:**

```bash
vyn run node server.js
vyn run docker compose up
```

---

## vyn check

Compare key sets between `.env` and `.env.example`.

```bash
vyn check
```

- Reports keys missing from `.env` (present in `.env.example`)
- Reports extra keys in `.env` (not in `.env.example`)
- Exits non-zero on any mismatch

---

## vyn history

List recorded sync snapshots.

```bash
vyn history
```

Shows timestamps and vault IDs from `.vyn/history/`.

---

## vyn doctor

Run local health checks.

```bash
vyn doctor
```

| Check | Passes when |
|---|---|
| `vault_directory` | `.vyn/` exists |
| `config_file` | `config.toml` is readable and valid TOML |
| `keychain` | project key loads from OS keychain |
| `manifest` | `.vyn/manifest.json` is readable and valid |
| `identity` | `.vyn/identity.toml` is valid and SSH key files exist |
| `relay_config` | `relay_url` is set and starts with `http://` or `https://` |
| `storage` | storage provider is `memory` or `relay` (not `unconfigured`) |

---

## vyn rotate

Rotate the project key and re-encrypt all remote state.

```bash
vyn rotate
```

1. Generates a new AES-256-GCM project key
2. Re-encrypts and re-uploads all tracked blobs and the manifest
3. Updates the OS keychain with the new key
4. Rebuilds invite files for known teammates
5. Writes a history entry

---

## vyn update

Check for a newer version and print upgrade instructions.

```bash
vyn update
```

Options:
- `--check` — only report whether a newer version is available, without printing upgrade instructions

**What it does:**

1. Fetches the latest published version from crates.io
2. Compares with the current binary version
3. If up to date, exits cleanly
4. If a newer version is available, prints instructions tailored to how vyn was installed (binary, cargo, or Docker)

**Example:**

```bash
vyn update
# vyn v0.1.3 is installed. v0.1.4 is available.
#
# Run the following to update:
#   curl -fsSL https://github.com/arnonsang/vyn/releases/latest/download/install.sh | sh
```

```bash
vyn update --check
# vyn v0.1.3 is installed. v0.1.4 is available.
```
