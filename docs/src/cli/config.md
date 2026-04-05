# vyn config

Configure storage and relay settings in `.vyn/config.toml`.

```bash
vyn config [options]
```

## Options

| Option | Description |
|---|---|
| `--provider <memory\|relay>` | Storage provider |
| `--relay-url <url>` | Relay URL (required when provider is `relay`) |
| `--non-interactive` | Skip interactive wizard; all required options must be provided via flags |

## Interactive mode (default)

```bash
vyn config
```

Opens a guided wizard to select the storage provider and, if relay is chosen, enter the relay URL.

## Non-interactive mode

```bash
# Memory (no persistence)
vyn config --provider memory --non-interactive

# Relay
vyn config --provider relay --relay-url https://relay.example.com --non-interactive
```

Useful in CI environments or automated setup scripts.

## Resulting config.toml

```toml
vault_id = "<uuid>"
project_name = "my-project"
storage_provider = "relay"
relay_url = "https://relay.example.com"
```

## Storage providers

| Provider | Description |
|---|---|
| `memory` | In-process only; state is lost when the process exits. Useful for testing. |
| `relay` | gRPC relay server. Requires `relay_url`. |
| `unconfigured` | Default after `vyn init`. Push/pull will fail until configured. |
