# vyn

**vyn** (En**v** + S**yn**c) is an encrypted env/config sync CLI for teams. It helps you encrypt, sync, diff, share, and run environment and configuration files with a local-first workflow and optional relay or S3 storage.

[![crates.io vyn-cli](https://img.shields.io/crates/v/vyn-cli?label=vyn-cli&color=fc8d62)](https://crates.io/crates/vyn-cli)
[![crates.io vyn-core](https://img.shields.io/crates/v/vyn-core?label=vyn-core&color=7fc97f)](https://crates.io/crates/vyn-core)
[![crates.io vyn-relay](https://img.shields.io/crates/v/vyn-relay?label=vyn-relay&color=beaed4)](https://crates.io/crates/vyn-relay)
[![CI](https://img.shields.io/github/actions/workflow/status/arnonsang/vyn/ci.yml?label=CI)](https://github.com/arnonsang/vyn/actions)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/arnonsang/vyn/blob/main/LICENSE)

## Highlights

- **AES-256-GCM encryption** — all blobs and manifests are encrypted before leaving the machine
- **GitHub OAuth Device Flow** — passwordless identity, no manual username entry
- **SSH-based key sharing** — invite teammates via [age](https://age-encryption.org) using their GitHub SSH public keys; invite embeds vault ID, relay URL, and key so recipients can onboard with a single command
- **`vyn.toml`** — non-secret public config committed to Git; makes `vyn clone` and CI pull work without manual configuration
- **One-step onboarding** — `vyn clone <relay_url> <vault_id>` finds invite, imports key, and pulls files automatically
- **Relay inspection** — `vyn relay status` and `vyn relay ls` to check connectivity and browse stored vaults/blobs
- **Local vault** — all metadata and keys live in `.vyn/` and are never committed to Git
- **Diff & status** — file-level and line-level diff against the encrypted baseline
- **Self-hosted relay** — run your own gRPC relay server with optional S3 mirroring
- **P2P stub** — `libp2p` module compiled into `vyn-core`, not yet CLI-exposed

## Project Layout

| Crate | Description |
|---|---|
| `vyn-core` | Crypto, keychain, manifest, storage, diff/merge, p2p |
| `vyn-cli` | End-user CLI command surface |
| `vyn-relay` | gRPC relay service implementation |
| `proto/vyn.proto` | Relay API contract (canonical source) |

## Current Status

Full MVP command set is implemented and tested:

- Local vault lifecycle: `init`, `st`, `diff`, `config`, `doctor`
- Sync: `push`, `pull`, `history`
- Identity + sharing: `auth` (OAuth + SSH verify), `share`, `link`, `clone`
- Key rotation: `rotate` (re-encrypts all remote state with a new project key)
- **v0.1.3:** `clone` (one-step onboarding), `relay status`, `relay ls`, `vyn.toml` public config, `update` (version check + upgrade instructions)
- Env management: `run`, `check`
- Relay server: `serve` with local and S3-mirror backends
- Docker / Docker Compose deployment ready

The P2P module (`vyn-core::p2p`) is compiled into the library but not yet exposed via CLI.
