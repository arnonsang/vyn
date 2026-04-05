# How Vyn Works

vyn uses two main flows: **identity + sharing**, and **push/pull sync**.

## Auth, Share, and Link

```mermaid
sequenceDiagram
  participant U as User
  participant C1 as vyn auth
  participant GH as GitHub
  participant FS as .vyn files
  participant C2 as vyn share
  participant C3 as vyn link
  participant KC as OS Keychain

  U->>C1: vyn auth
  C1->>GH: OAuth Device Flow (required)
  GH-->>C1: verified github_username
  C1->>GH: GET /<username>.keys
  GH-->>C1: registered SSH public keys
  C1->>C1: ssh-keygen challenge-response (prove key ownership)
  C1->>FS: write .vyn/identity.toml

  U->>C2: vyn share @teammate
  C2->>KC: load project key
  C2->>GH: GET /teammate.keys
  GH-->>C2: SSH public keys
  C2->>FS: write encrypted invites (.age)

  U->>C3: vyn link <vault_id>
  C3->>FS: read invite + identity private key path
  C3->>KC: store linked project key
  C3->>FS: rewrite vault_id in config.toml
```

## Push/Pull with Relay Storage

```mermaid
sequenceDiagram
  participant U as User
  participant CLI as vyn push/pull
  participant Blobs as .vyn/blobs/
  participant KC as OS Keychain
  participant Relay as Relay API
  participant S3 as S3 mirror (optional)

  U->>CLI: vyn push
  CLI->>KC: load project key
  CLI->>Blobs: encrypt file → {hash}.enc (skip if cached)
  CLI->>Relay: upload encrypted blobs + encrypted manifest
  Relay->>S3: mirror write (optional)
  CLI->>CLI: update manifest.json + history

  U->>CLI: vyn pull
  CLI->>KC: load project key
  CLI->>Relay: download encrypted manifest + blobs
  Relay->>S3: fetch if local object missing (optional)
  CLI->>Blobs: cache encrypted blob as {hash}.enc
  CLI->>CLI: decrypt manifest + blobs in memory
  CLI->>CLI: write plaintext files + update manifest + history
```

## Key properties

- **Zero-knowledge relay** — the relay and S3 backend never see plaintext content or metadata
- **Content-addressed blobs** — files are stored as `{sha256}.enc`; identical content is deduplicated
- **Idempotent push** — blobs already cached locally are not re-encrypted or re-uploaded
- **In-memory decryption** — `vyn pull` never writes plaintext anywhere except the final destination path
