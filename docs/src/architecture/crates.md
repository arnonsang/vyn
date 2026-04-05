# Crate Structure

vyn is a Cargo workspace with three crates.

## Dependency graph

```
vyn-cli  ──────►  vyn-core  ──────►  vyn-relay
           └────────────────────►  vyn-relay
```

`vyn-cli` depends on both `vyn-core` (library logic) and `vyn-relay` (for `vyn serve --relay`). `vyn-core` depends on `vyn-relay` for the `RelayStorageProvider` gRPC client.

## vyn-core

Shared library crate. Contains all cryptographic primitives and storage abstractions.

| Module | Responsibility |
|---|---|
| `crypto` | AES-256-GCM encrypt/decrypt, key generation, `SecretBytes` type |
| `keychain` | OS keychain wrapper via keyring-rs |
| `manifest` | `Manifest` + `FileEntry` structs, versioning, serde |
| `blob` | Encrypt file → content-addressed `blobs/<sha256>.enc` |
| `diff` | Binary detection + text diff via `similar` |
| `merge` | Auto-merge + conflict markers (`<<<<<<< LOCAL` / `>>>>>>> REMOTE`) |
| `ignore` | `.vynignore` parsing via the `ignore` crate |
| `storage` | `StorageProvider` trait + `InMemoryStorageProvider` |
| `relay_storage` | `RelayStorageProvider` — gRPC client implementing `StorageProvider` |
| `models` | `VaultConfig`, `IdentityConfig`, `HistoryEntry` |
| `wrapping` | age-based SSH key wrapping (RSA + Ed25519) |
| `p2p` | libp2p stub (compiled, not CLI-exposed) |

## vyn-cli

Binary crate. Thin command dispatch layer over `vyn-core`.

| File | Description |
|---|---|
| `main.rs` | `clap` `Commands` enum with all subcommands |
| `output.rs` | Colored terminal rendering helpers |
| `commands/init.rs` | `vyn init` |
| `commands/auth.rs` | `vyn auth` — GitHub OAuth + SSH verify |
| `commands/config.rs` | `vyn config` |
| `commands/push.rs` | `vyn push` |
| `commands/pull.rs` | `vyn pull` |
| `commands/status.rs` | `vyn st` |
| `commands/diff.rs` | `vyn diff` |
| `commands/share.rs` | `vyn share` |
| `commands/link.rs` | `vyn link` |
| `commands/serve.rs` | `vyn serve --relay` |
| `commands/run.rs` | `vyn run` |
| `commands/check.rs` | `vyn check` |
| `commands/history.rs` | `vyn history` |
| `commands/doctor.rs` | `vyn doctor` |
| `commands/rotate.rs` | `vyn rotate` |
| `commands/add.rs` | `vyn add` |
| `commands/del.rs` | `vyn del` |

## vyn-relay

Relay server library. Can be embedded as a Rust library or run via `vyn serve`.

| Module | Description |
|---|---|
| `lib.rs` | `serve()`, `serve_with_config()`, `serve_with_listener()`, `ServeConfig` |
| `service.rs` | `RelayService` — tonic gRPC service implementation |
| `store.rs` | `FileStore` — file-based blob/manifest/invite storage + S3 mirror |
| `auth.rs` | SSH challenge-response authentication verification |

## Key dependencies

| Crate | Purpose |
|---|---|
| `ring` | AES-256-GCM, secure random |
| `age` (ssh feature) | SSH key wrapping |
| `keyring` | OS keychain |
| `similar` | Text diffing |
| `tokio` | Async runtime |
| `tonic` + `prost` | gRPC client + server |
| `aws-sdk-s3` | S3/R2 storage backend |
| `serde` + `serde_json` + `toml` | Serialization |
| `sha2` | Content-addressing |
| `secrecy` | Zeroize secrets in memory |
| `reqwest` | HTTP client (GitHub API, OAuth) |
| `clap` | CLI argument parsing |
| `ignore` | .vynignore parsing |
| `libp2p` | P2P networking (stub, not CLI-exposed) |
