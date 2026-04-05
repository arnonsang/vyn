# Storage Backends

## local-only

Blobs and manifests are persisted to the relay's data directory on disk.

**Enable:** `vyn serve --relay --data-dir /path/to/data`

| Property | Value |
|---|---|
| Write | Persist to relay volume |
| Read | Read from relay volume |
| Best fit | Simplest self-hosted setup |

---

## local + S3 mirror

Blobs are written locally first, then mirrored to S3 (or any S3-compatible API like Cloudflare R2 or MinIO).

**Enable:** add `--s3-bucket` and `--s3-region`

```bash
vyn serve --relay \
  --data-dir ./.vyn-relay \
  --s3-bucket my-vyn-bucket \
  --s3-region us-east-1 \
  --s3-endpoint https://s3.us-east-1.amazonaws.com \
  --s3-prefix vyn
```

| Property | Value |
|---|---|
| Write | Persist locally first, then mirror to S3 |
| Read | Local cache; fallback to S3 if object missing |
| Best fit | Durability + offsite cloud copy |

If S3 is unavailable, the relay falls back to local persistence automatically.

## Choosing a backend

| Scenario | Recommendation |
|---|---|
| Small team, single server | local-only |
| High availability / offsite backup | local + S3 mirror |
| Cloudflare R2 (no egress fees) | local + S3 mirror with `--s3-endpoint` set to R2 endpoint |
| MinIO self-hosted | local + S3 mirror with `--s3-endpoint` set to MinIO URL |
