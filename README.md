<p align="center">
  <img src="https://raw.githubusercontent.com/arnonsang/vyn/fdf6641affa94e39d08f06afa90912e8b0bb92d0/assets/logo_light_transparent.png" alt="vyn logo" width="100" />
  <br />
  <h1 align="center">vyn</h1>
  <p align="center">Encrypted env/config sync CLI for teams. Helps you encrypt, sync, diff, share, and run environment/config files with a local-first workflow and optional relay/S3 storage.</p>
  <p align="center">
    <a href="https://crates.io/crates/vyn-cli"><img src="https://img.shields.io/crates/v/vyn-cli?label=vyn-cli&color=fc8d62" alt="crates.io vyn-cli" /></a>
    <a href="https://crates.io/crates/vyn-core"><img src="https://img.shields.io/crates/v/vyn-core?label=vyn-core&color=7fc97f" alt="crates.io vyn-core" /></a>
    <a href="https://crates.io/crates/vyn-relay"><img src="https://img.shields.io/crates/v/vyn-relay?label=vyn-relay&color=beaed4" alt="crates.io vyn-relay" /></a>
    <a href="https://github.com/arnonsang/vyn/actions"><img src="https://img.shields.io/github/actions/workflow/status/arnonsang/vyn/ci.yml?label=CI" alt="CI" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="license" /></a>
  </p>
</p>

> **This project is under active development.** Until v1.0.0 is released, any version bump may include breaking changes to the CLI interface, config file format, relay protocol, or storage layout. Pin your version if you depend on stable behavior.

**Full documentation:** [vyn.iamick.dev](https://vyn.iamick.dev)

## Highlights

- AES-256-GCM encryption for all synced blobs and manifests
- GitHub OAuth Device Flow identity (no passwords, no manual username entry)
- SSH-based project key sharing via age - invite embeds vault ID, relay URL, and key so recipients onboard in one command
- `vyn.toml` - non-secret public config committed to Git; enables zero-config `vyn pull` and `vyn clone`
- One-step onboarding: `vyn clone` - finds invite, imports key, pulls all files
- Opt-in `.vynignore` - track only what you explicitly include
- Relay session token caching - no re-auth on every command after `vyn auth`
- Self-hosted relay server with optional S3 mirroring

## Install

### Option A: crates.io (recommended)

```bash
cargo install vyn-cli
```

### Option B: Pre-built binary

```bash
# Linux / macOS
curl -fsSL https://github.com/arnonsang/vyn/releases/latest/download/install.sh | sh

# Windows: download vyn-x86_64-pc-windows-msvc.zip from releases
```

### Option C: Build from source

```bash
git clone https://github.com/arnonsang/vyn.git
cd vyn
cargo install --path crates/vyn-cli
```

See [Installation](https://vyn.iamick.dev/getting-started/installation.html) for Docker build and uninstall options.

## Quick Start

```bash
vyn init my-project   # create vault
vyn config            # configure relay storage
vyn auth              # GitHub OAuth + SSH verify
vyn push              # encrypt and upload
vyn pull              # download and decrypt
```

### Join an existing vault

```bash
mkdir my-project && cd my-project
vyn clone https://relay.example.com <vault_id>
```

## Project Layout

- `crates/vyn-core` - crypto, keychain, manifest, storage, diff/merge, p2p
- `crates/vyn-cli` - end-user CLI command surface
- `crates/vyn-relay` - gRPC relay service implementation
- `proto/vyn.proto` - relay API contract

## Documentation

- [Getting Started](https://vyn.iamick.dev/getting-started/quick-start.html)
- [CLI Reference](https://vyn.iamick.dev/cli/init.html)
- [Config Files](https://vyn.iamick.dev/configuration/files.html)
- [Relay Deployment](https://vyn.iamick.dev/relay/overview.html)
- [Security Notes](https://vyn.iamick.dev/concepts/security.html)
- [How Vyn Works](https://vyn.iamick.dev/concepts/overview.html)
