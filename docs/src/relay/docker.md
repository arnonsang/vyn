# Docker Deployment

## Run directly

```bash
docker run --rm -p 50051:50051 -v vyn-relay-data:/data \
  ghcr.io/arnonsang/vyn-relay:latest \
  --relay --port 50051 --data-dir /data
```

Using environment variables:

```bash
docker run --rm -p 50051:50051 -v vyn-relay-data:/data \
  -e VYN_RELAY_PORT=50051 \
  -e VYN_RELAY_DATA_DIR=/data \
  -e VYN_RELAY_S3_BUCKET=my-vyn-bucket \
  -e VYN_RELAY_S3_REGION=us-east-1 \
  ghcr.io/arnonsang/vyn-relay:latest --relay
```

## Docker Compose

A `docker-compose.yml` is included in the repository:

```bash
VYN_RELAY_PORT=50052 docker compose up -d --build
```

Relay data is stored in the named volume `vyn-relay-data`.

## Build from source

```bash
git clone https://github.com/arnonsang/vyn.git
cd vyn
docker build -t vyn-relay .
docker run --rm -p 50051:50051 -v vyn-relay-data:/data vyn-relay \
  --relay --port 50051 --data-dir /data
```

## TLS with Caddy (example)

```yaml
# Caddyfile
relay.example.com {
  reverse_proxy h2c://10.0.0.2:50051
}
```

Caddy handles HTTPS and forwards plain gRPC (h2c) to the relay container. Set `relay_url = "https://relay.example.com"` in clients.
