# Environment Variables

## CLI environment variables

| Variable | Used by | Description |
|---|---|---|
| `VYN_GITHUB_CLIENT_ID` | `vyn auth` | Override the built-in OAuth `client_id` (advanced) |
| `VYN_SKIP_GITHUB_VERIFY=1` | `vyn auth` | Skip GitHub identity and SSH verification (offline CI; debug builds only) |
| `VYN_RELAY_PORT` | `vyn serve`, Docker Compose | Relay listening port override |

## Relay environment variables

These configure the relay server when running via `vyn serve`, Docker, or Docker Compose.

| Variable | Default | Description |
|---|---|---|
| `VYN_RELAY_PORT` | `50051` | Listening port |
| `VYN_RELAY_DATA_DIR` | `./.vyn-relay` | Local persistence directory |
| `VYN_RELAY_S3_BUCKET` | *(none)* | S3 mirror bucket (optional) |
| `VYN_RELAY_S3_REGION` | *(none)* | S3 region (required if bucket set) |
| `VYN_RELAY_S3_ENDPOINT` | *(none)* | Custom S3 endpoint for R2/MinIO (optional) |
| `VYN_RELAY_S3_PREFIX` | *(none)* | Key prefix inside bucket (optional) |

CLI flags override environment variables; environment variables override defaults.
