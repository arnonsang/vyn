# Config Files

## vyn.toml

Public vault config, committed to Git. Written by `vyn init` and updated by `vyn config`.

```toml
# vyn.toml -- commit this file
vault_id = "f47ac10b-58cc-4372-a567-0e02b2c3d479"
relay_url = "https://relay.example.com"
```

| Field | Description |
|---|---|
| `vault_id` | UUID identifying this vault on the storage backend |
| `relay_url` | URL of the relay server |

This file contains no secrets. Committing it lets teammates (and `vyn clone`) discover the vault's relay and ID without any manual communication.

> **Note:** `push` and `pull` fall back to `vyn.toml` if `.vyn/config.toml` is missing, so you can run `vyn pull` immediately after cloning a repo without any extra setup.

---

## .vyn/config.toml

Private vault configuration. Written by `vyn init` and updated by `vyn config` and `vyn link`. **Not committed to Git** (`.vyn/` is added to `.gitignore` by `vyn init`).

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

`vyn auth` also writes an identical file to `~/.vyn/identity.toml`. This global copy is used by `vyn clone` when starting fresh in a new directory where no local `.vyn/` exists yet.

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

---

## ~/.config/vyn/global.toml

Global per-user config (XDG-compliant). Written automatically by `vyn update` and read at startup to cache install-method detection.

```toml
install_method = "binary"   # binary | cargo | docker
installed_version = "0.1.3"
```

| Field | Description |
|---|---|
| `install_method` | How `vyn` was installed — controls which upgrade command `vyn update` prints |
| `installed_version` | Version string recorded at last update check |

This file is managed automatically; you do not normally need to edit it.
