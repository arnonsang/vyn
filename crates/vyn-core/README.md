# vyn-core

Core library powering [vyn](https://github.com/arnonsang/vyn) — the encrypted env/config sync CLI for teams.

This crate is not meant to be used directly. For the end-user CLI, see [`vyn-cli`](https://crates.io/crates/vyn-cli).

## What's in here

- **Crypto** — AES-256-GCM encryption/decryption for blobs and manifests
- **Keychain** — project key storage and retrieval via the OS keychain
- **Manifest** — filesystem scanning, hashing, and manifest capture
- **Storage** — local blob store and relay storage provider abstraction
- **Diff/Merge** — line-level diff and 3-way merge engine
- **Wrapping** — SSH key-based wrapping/unwrapping of project keys via `age`
- **P2P** — libp2p-based local discovery module (experimental)

## Crates in this workspace

| Crate | Description |
|---|---|
| [`vyn-cli`](https://crates.io/crates/vyn-cli) | End-user CLI — install this |
| [`vyn-core`](https://crates.io/crates/vyn-core) | Core library (this crate) |
| [`vyn-relay`](https://crates.io/crates/vyn-relay) | Self-hosted gRPC relay server |

## License

MIT
