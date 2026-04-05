# vyn-relay

Self-hosted gRPC relay server for [vyn](https://github.com/arnonsang/vyn) — the encrypted env/config sync CLI for teams.

This crate is the relay server library. For the end-user CLI, see [`vyn-cli`](https://crates.io/crates/vyn-cli).

## What's in here

- gRPC service implementation (authentication, manifest CRUD, blob streaming, invites)
- File-based local persistence store
- Optional S3 mirroring backend
- SSH-based challenge/response identity verification

## Running the relay

The relay is embedded in `vyn-cli` and started automatically, but you can also run it standalone:

```rust
vyn_relay::serve(50051, "./data".to_string()).await?;
```

Or with S3 mirroring:

```rust
vyn_relay::serve_with_config(50051, "./data".to_string(), ServeConfig {
    s3_bucket: Some("my-bucket".into()),
    s3_region: Some("us-east-1".into()),
    ..Default::default()
}).await?;
```

## Crates in this workspace

| Crate | Description |
|---|---|
| [`vyn-cli`](https://crates.io/crates/vyn-cli) | End-user CLI — install this |
| [`vyn-core`](https://crates.io/crates/vyn-core) | Core library |
| [`vyn-relay`](https://crates.io/crates/vyn-relay) | Self-hosted gRPC relay server (this crate) |

## License

MIT
