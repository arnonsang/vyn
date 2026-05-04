# vyn

**vyn** (En**v** + S**yn**c) is an encrypted env/config sync CLI for teams. It helps you encrypt, sync, diff, share, and run environment and configuration files with a local-first workflow and optional relay or S3 storage.

[![crates.io vyn-cli](https://img.shields.io/crates/v/vyn-cli?label=vyn-cli&color=fc8d62)](https://crates.io/crates/vyn-cli)
[![crates.io vyn-core](https://img.shields.io/crates/v/vyn-core?label=vyn-core&color=7fc97f)](https://crates.io/crates/vyn-core)
[![crates.io vyn-relay](https://img.shields.io/crates/v/vyn-relay?label=vyn-relay&color=beaed4)](https://crates.io/crates/vyn-relay)
[![CI](https://img.shields.io/github/actions/workflow/status/arnonsang/vyn/ci.yml?label=CI)](https://github.com/arnonsang/vyn/actions)
[![license](https://img.shields.io/badge/license-MIT-blue)](https://github.com/arnonsang/vyn/blob/main/LICENSE)

## Highlights

- **AES-256-GCM encryption** - all blobs and manifests are encrypted before leaving the machine
- **GitHub OAuth Device Flow** - passwordless identity, no manual username entry
- **SSH-based key sharing** - invite teammates via [age](https://age-encryption.org) using their GitHub SSH public keys; invite embeds vault ID, relay URL, and key so recipients can onboard with a single command
- **`vyn.toml`** - non-secret public config committed to Git; makes `vyn clone` and CI pull work without manual configuration
- **One-step onboarding** - `vyn clone <relay_url> <vault_id>` finds invite, imports key, and pulls files automatically
- **Relay session token caching** - auth once with `vyn auth`, subsequent commands reuse the cached token (24h TTL)
- **Opt-in `.vynignore`** - everything ignored by default; only explicitly included patterns are tracked
- **Relay inspection** - `vyn relay status` and `vyn relay ls` to check connectivity and browse stored vaults/blobs
- **Local vault** - all metadata and keys live in `.vyn/` and are never committed to Git
- **Diff & status** - file-level and line-level diff against the encrypted baseline
- **Self-hosted relay** - run your own gRPC relay server with optional S3 mirroring
- **P2P stub** - `libp2p` module compiled into `vyn-core`, not yet CLI-exposed

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
- **v0.1.4:** relay session token caching (no re-auth on every command), opt-in `.vynignore` model, relay session TTL (24h), auth progress spinners on all commands
- Env management: `run`, `check`
- Relay server: `serve` with local and S3-mirror backends
- Docker / Docker Compose deployment ready

The P2P module (`vyn-core::p2p`) is compiled into the library but not yet exposed via CLI.

## Planned Improvements

### Security / Privacy

- `vyn revoke @user` - remove a teammate's invite from the relay and optionally trigger key rotation
- Invite expiry - time-bound invites (`--expires 7d`) so stale entries on the relay don't accumulate
- Relay audit log - record who authenticated and which operations ran (no plaintext logged)
- Passphrase-protected vault - derive PK via Argon2 as an alternative to the OS keychain

### Onboarding / Team UX

- `vyn whoami` - print current identity: github username, SSH key path, relay URL, vault ID
- `vyn team` - list who has been granted access (reads invite list from relay)
- `vyn invite` link - generate a short token URL a teammate can paste for one-click `vyn clone`

### Storage / Transport

- P2P mode - complete the `libp2p` stub with mDNS discovery + Gossipsub for zero-latency LAN sync
- `vyn conflicts` - list and interactively resolve conflict-marker files left by `vyn pull`
- Selective push/pull - `vyn push .env.production` / `vyn pull .env.staging` for single-file sync

### CI / Automation

- `vyn env print` - dump decrypted key=value to stdout for CI env injection without subprocess exec
- Non-interactive auth - `vyn auth --token <github_pat>` for headless CI environments
- GitHub Actions action - `uses: arnonsang/vyn-action@v1` wrapping install + auth + pull

### Developer Experience

- Shell completions - `vyn completions bash|zsh|fish`
- `vyn config --edit` - open `.vyn/config.toml` in `$EDITOR` directly
- Progress bars on push/pull - show bytes transferred for large blob sets
- `vyn add` interactive prompt - show which files will be tracked vs. ignored before writing `.vynignore`
