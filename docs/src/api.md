# API Reference

vyn is published as three crates on [crates.io](https://crates.io). Full rustdoc is available on [docs.rs](https://docs.rs).

## vyn-core

[![docs.rs](https://img.shields.io/docsrs/vyn-core)](https://docs.rs/vyn-core)
[![crates.io](https://img.shields.io/crates/v/vyn-core)](https://crates.io/crates/vyn-core)

Core library providing all cryptographic primitives and storage abstractions.

**Key modules:**

| Module | Description |
|---|---|
| `crypto` | AES-256-GCM encrypt/decrypt, key generation, `SecretBytes` |
| `keychain` | OS keychain wrapper (keyring-rs) |
| `manifest` | `Manifest` + `FileEntry`, versioning, serde |
| `blob` | Content-addressed encrypted blob storage |
| `diff` | Binary detection + text diff via `similar` |
| `merge` | Auto-merge + conflict markers |
| `ignore` | `.vynignore` parsing |
| `storage` | `StorageProvider` trait + `InMemoryStorageProvider` |
| `relay_storage` | `RelayStorageProvider` (gRPC client) |
| `models` | `VaultConfig`, `IdentityConfig`, `HistoryEntry` |
| `wrapping` | age-based SSH key wrapping |
| `p2p` | libp2p stub (not CLI-exposed) |

[View on docs.rs →](https://docs.rs/vyn-core)

---

## vyn-relay

[![docs.rs](https://img.shields.io/docsrs/vyn-relay)](https://docs.rs/vyn-relay)
[![crates.io](https://img.shields.io/crates/v/vyn-relay)](https://crates.io/crates/vyn-relay)

Relay server library. Embed the relay directly as a Rust library or run it via `vyn serve --relay`.

**Key exports:**

| Symbol | Description |
|---|---|
| `serve()` | Start relay with default config |
| `serve_with_config(ServeConfig)` | Start relay with custom config |
| `serve_with_listener(TcpListener, ServeConfig)` | Start relay on a pre-bound listener |
| `ServeConfig` | Port, data dir, S3 config |

[View on docs.rs →](https://docs.rs/vyn-relay)

---

## vyn-cli

[![crates.io](https://img.shields.io/crates/v/vyn-cli)](https://crates.io/crates/vyn-cli)

End-user binary crate. Not intended as a library — install with `cargo install vyn-cli`.
