# Installation

## Option A: Install from crates.io (recommended)

```bash
cargo install vyn-cli
vyn --help
```

Requires Rust 1.80+ with Edition 2024 support. Install Rust via [rustup.rs](https://rustup.rs).

## Option B: Build from source

```bash
git clone https://github.com/arnonsang/vyn.git
cd vyn
cargo install --path crates/vyn-cli
vyn --help
```

## Verify installation

```bash
vyn --version
```

## OS Keychain requirements

`vyn` stores your project key in the OS keychain. No extra setup is needed on macOS or Windows.

| Platform | Backend |
|---|---|
| Linux | `keyutils` (kernel keyring) or Secret Service (D-Bus) |
| macOS | macOS Keychain |
| Windows | Windows Credential Manager (DPAPI) |

On Linux with Secret Service, ensure `libdbus-1-dev` (Debian/Ubuntu) or `dbus-devel` (Fedora) is installed at compile time.
