# Config Files

## .vyn/config.toml

Primary vault configuration. Written by `vyn init` and updated by `vyn config` and `vyn link`.

```toml
vault_id = "f47ac10b-58cc-4372-a567-0e02b2c3d479"
project_name = "my-project"
storage_provider = "relay"        # memory | relay | unconfigured
relay_url = "https://relay.example.com"
```

| Field | Description |
|---|---|
| `vault_id` | UUID identifying this vault on the storage backend |
| `project_name` | Human-readable project name |
| `storage_provider` | Active backend: `memory`, `relay`, or `unconfigured` |
| `relay_url` | URL of the relay server (required when provider is `relay`) |

---

## .vyn/identity.toml

Written by `vyn auth`. Stores your confirmed GitHub identity and SSH key paths.

```toml
github_username = "your-handle"
ssh_private_key = "/home/you/.ssh/id_ed25519"
ssh_public_key  = "/home/you/.ssh/id_ed25519.pub"
```

| Field | Description |
|---|---|
| `github_username` | Verified GitHub username (confirmed via OAuth) |
| `ssh_private_key` | Absolute path to the SSH private key used for key unwrapping |
| `ssh_public_key` | Absolute path to the corresponding SSH public key |

---

## .vyn/manifest.json

Local plaintext manifest. Tracks the baseline state of all vault files.

```json
{
  "version": 1,
  "files": [
    { "path": ".env", "sha256": "a1b2c3...", "size": 512, "mode": 33188 }
  ]
}
```

This file is the reference point for `vyn st` and `vyn diff`. It is updated on every `vyn push` and `vyn pull`.

---

## .vynignore

Exclude files from vault tracking. Uses standard gitignore syntax.

```gitignore
# .vynignore
*.log
build/
dist/
node_modules/
```
