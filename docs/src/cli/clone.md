# vyn clone

Clone a vault from a relay onto this machine in one step.

```bash
vyn clone <relay_url> <vault_id>
```

| Argument | Description |
|---|---|
| `relay_url` | URL of the relay server hosting the vault |
| `vault_id` | UUID of the vault to clone |

## Prerequisites

- Run `vyn auth` at least once on this machine. `vyn clone` reads your identity from `~/.vyn/identity.toml` so you do not need to be inside any particular directory.
- A teammate must have run `vyn share @you` so an invite is waiting on the relay.

## What it does

1. Reads `~/.vyn/identity.toml` (global) or `.vyn/identity.toml` (local)
2. Creates `.vyn/` in the current directory and copies the identity there
3. Authenticates with the relay using your SSH key
4. Fetches and decrypts the invite for your GitHub username
5. Stores the vault's project key in the OS keychain
6. Writes `.vyn/config.toml` and `vyn.toml` (with `vault_id` and `relay_url` embedded) — no manual configuration needed
7. Runs `vyn pull` to download and decrypt all vault files

## Example

```bash
mkdir my-project && cd my-project
vyn clone https://relay.example.com f47ac10b-58cc-4372-a567-0e02b2c3d479
```

```
  vyn clone
  ✔ authenticated
  ✔ 1 invite(s) found
  ✔ invite decrypted
  ✔ vault f47ac10b-58cc-4372-a567-0e02b2c3d479 linked

  identity   @you
  relay      https://relay.example.com
  key stored OS keychain

  next       pulling files…

  vyn pull
  ✔ 3 files in manifest
  ✔ blobs written to disk [██████████████] 3/3
  ✔ vault f47ac10b-58cc-4372-a567-0e02b2c3d479 pulled

  files      3 synced
```

## vyn.toml shortcut

If a repo already has a committed `vyn.toml` you can read the `relay_url` and `vault_id` from it directly:

```bash
cat vyn.toml
# vault_id  = "f47ac10b-58cc-4372-a567-0e02b2c3d479"
# relay_url = "https://relay.example.com"

vyn clone https://relay.example.com f47ac10b-58cc-4372-a567-0e02b2c3d479
```

## vs. vyn link

| | `vyn clone` | `vyn link` |
|---|---|---|
| Requires existing `.vyn/` | No | Yes |
| Reads global identity | Yes | No |
| Pulls files automatically | Yes | No |
| Best for | Fresh machines / new repos | Adding access within an existing checkout |
