# Relay Reference

## CLI flags

```
vyn serve --relay [options]
```

| Flag | Default | Description |
|---|---|---|
| `--relay` | — | Required. Enable relay mode. |
| `--port` | `50051` | Listening port |
| `--data-dir` | `./.vyn-relay` | Local persistence directory |
| `--s3-bucket` | *(none)* | S3 mirror bucket |
| `--s3-region` | *(none)* | S3 region (required if bucket set) |
| `--s3-endpoint` | *(none)* | Custom S3 endpoint |
| `--s3-prefix` | *(none)* | Key prefix inside bucket |

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `VYN_RELAY_PORT` | `50051` | Listening port |
| `VYN_RELAY_DATA_DIR` | `./.vyn-relay` | Data directory |
| `VYN_RELAY_S3_BUCKET` | *(none)* | S3 mirror bucket |
| `VYN_RELAY_S3_REGION` | *(none)* | S3 region |
| `VYN_RELAY_S3_ENDPOINT` | *(none)* | Custom S3 endpoint |
| `VYN_RELAY_S3_PREFIX` | *(none)* | Key prefix |

CLI flags override environment variables; environment variables override defaults.

## gRPC service

The relay exposes the `VynRelay` gRPC service on the configured port. See [gRPC Protocol](../architecture/protocol.md) for the full RPC reference.

## Client configuration

Set `storage_provider = "relay"` and `relay_url = "<url>"` in `.vyn/config.toml`:

```toml
storage_provider = "relay"
relay_url = "https://relay.example.com"
```
