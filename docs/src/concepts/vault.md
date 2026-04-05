# Vault Structure

A vault is a project-scoped collection of encrypted state. Each vault has a unique UUID (`vault_id`) and lives in the `.vyn/` directory at the root of your project.

## Directory layout

```
.vyn/
├── config.toml         # vault_id, project name, storage provider settings
├── identity.toml       # github_username, SSH key paths (written by vyn auth)
├── manifest.json       # local plaintext manifest (path, sha256, size, mode, version)
├── history/            # local sync snapshot log
├── blobs/              # local cache of encrypted blobs
│   └── <sha256>.enc    # AES-256-GCM encrypted file content
└── invites/            # encrypted project key invites
    └── <vault_id>__<user>__<n>.age
```

## config.toml

```toml
vault_id = "<uuid>"
project_name = "my-project"
storage_provider = "relay"       # memory | relay | unconfigured
relay_url = "https://relay.example.com"
```

## identity.toml

Written by `vyn auth`. Contains your confirmed GitHub identity and the SSH key paths used for key wrapping.

```toml
github_username = "your-handle"
ssh_private_key = "/home/you/.ssh/id_ed25519"
ssh_public_key  = "/home/you/.ssh/id_ed25519.pub"
```

## Git safety

`vyn init` automatically adds `.vyn/` to `.gitignore`. The vault directory should never be committed — it holds your encrypted blobs and identity configuration.

## .vynignore

Create a `.vynignore` file in your project root to exclude files from tracking. The syntax is identical to `.gitignore`.

```
# .vynignore
*.log
build/
dist/
```

`vyn init` copies `.vynignore.example` to `.vynignore` if an example file is present.
