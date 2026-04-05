# Relay Overview

The vyn relay is a **self-hosted gRPC server** that stores encrypted blobs and manifests on behalf of vault clients. It is the recommended storage backend for teams.

## Key properties

- **Zero-knowledge** — the relay stores only ciphertext; it cannot read your files or project key
- **Authentication** — clients authenticate via SSH challenge-response (registered during `vyn auth`)
- **Optional S3 mirror** — blobs can be mirrored to S3/R2 for durability
- **Single binary** — run via `vyn serve --relay` (same binary as the CLI)

## Running the relay

```bash
# Basic
vyn serve --relay --port 50051 --data-dir ./.vyn-relay

# With S3 mirror
vyn serve --relay \
  --port 50051 \
  --data-dir ./.vyn-relay \
  --s3-bucket my-vyn-bucket \
  --s3-region us-east-1
```

## Relay data layout

```
<data-dir>/
├── manifests/<vault_id>.enc
├── blobs/<vault_id>/<sha256>.enc
├── identities/<username>.pub
└── invites/<vault_id>/<recipient>/<sha256>.age
```

## TLS

The relay does not terminate TLS itself. Use a reverse proxy (nginx, Caddy) in front of the relay port for HTTPS.

```nginx
server {
    listen 443 ssl http2;
    ssl_certificate     /etc/ssl/certs/relay.crt;
    ssl_certificate_key /etc/ssl/private/relay.key;

    location / {
        grpc_pass grpc://127.0.0.1:50051;
    }
}
```

Set `relay_url = "https://relay.example.com"` in `.vyn/config.toml`.
