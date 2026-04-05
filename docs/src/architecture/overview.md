# System Design

## Goals

- **Zero-knowledge security** — all data is encrypted locally before leaving the machine
- **Hybrid transport** — relay (gRPC) or P2P (libp2p, not yet CLI-exposed)
- **Local-first** — vault works fully offline; relay is optional persistence
- **Single binary** — `vyn` is both the CLI client and the relay server

## High-level architecture

```
                         ┌────────────────┐
                         │   vyn-cli      │
                         │  (clap CLI)    │
                         └─────┬─────────┘
                               │ depends on
               ┌───────────┴───────────┐
               │        vyn-core         │
               │  crypto / keychain /    │
               │  manifest / diff /      │
               │  storage / wrapping     │
               └─────────┬──────────┘
                         │ gRPC client
               ┌─────────┴──────────┐
               │       vyn-relay         │
               │  gRPC server / store /  │
               │  auth / S3 mirror       │
               └───────────────────┘
```

## Data flow summary

| Operation | Local | Relay |
|---|---|---|
| `vyn push` | Encrypt blobs + manifest with PK | Store ciphertext only |
| `vyn pull` | Decrypt in memory, write to disk | Serve ciphertext |
| `vyn share` | Wrap PK with recipient SSH key | Store wrapped invite |
| `vyn auth` | Prove SSH key ownership | Register public key |

## Security boundary

The encryption/decryption boundary is always the local machine. The relay is a dumb ciphertext store. It authenticates clients (to prevent unauthorized writes) but never has access to the plaintext or the project key.
