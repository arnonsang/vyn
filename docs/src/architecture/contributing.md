# Contributing

## Prerequisites

- Rust (Edition 2024, stable)
- `protoc` is **not** required — `vyn-relay` uses `protoc-bin-vendored` which vendors the compiler

## Build

```bash
git clone https://github.com/arnonsang/vyn.git
cd vyn
cargo build
```

Build the release binary:

```bash
cargo build --release
# binary: target/release/vyn
```

## Test

```bash
cargo test --workspace
```

Relay integration tests use the `test-bypass-auth` feature to skip SSH challenge-response:

```bash
cargo test -p vyn-core --features vyn-relay/test-bypass-auth
```

## Workspace layout

```
vyn/
├── Cargo.toml           # workspace root
├── proto/vyn.proto      # canonical gRPC definition
└── crates/
    ├── vyn-core/
    ├── vyn-cli/
    └── vyn-relay/
```

## Updating the proto

Edit `proto/vyn.proto` (the canonical source). Regeneration happens automatically at build time via `vyn-relay/build.rs` using `tonic-build`.

After editing, copy the updated proto to the bundled path:

```bash
cp proto/vyn.proto crates/vyn-relay/proto/vyn.proto
```

## Code style

- `cargo fmt` before committing
- `cargo clippy --workspace` — no warnings
- All crypto paths go through `vyn-core::crypto` — do not call `ring` APIs directly in `vyn-cli`

## Docs

This documentation site is built with [mdBook](https://rust-lang.github.io/mdBook/). Sources are in `docs/src/`.

```bash
# Install mdbook
cargo install mdbook mdbook-mermaid

# Serve locally
cd docs && mdbook serve
```
