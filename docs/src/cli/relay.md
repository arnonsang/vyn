# vyn relay

Relay inspection subcommands. Requires an authenticated relay connection (run `vyn auth` first).

---

## vyn relay status

Check connectivity and authentication against the configured relay.

```bash
vyn relay status
```

Reads `relay_url` from `.vyn/config.toml` or `vyn.toml`, authenticates using `.vyn/identity.toml`, and prints the result.

**Example output:**

```
  relay    http://localhost:50051
  identity @arnonsang (ssh-ed25519:my-project)
  auth     OK
```

---

## vyn relay ls [vault_id]

List vaults and blobs stored on the relay.

```bash
# List all vaults you have access to
vyn relay ls

# List blobs inside a specific vault
vyn relay ls <vault_id>
```

| Argument | Description |
|---|---|
| `vault_id` | (optional) If given, list blobs inside that specific vault |

**Example — list all vaults:**

```
Vaults:
  f47ac10b-58cc-4372-a567-0e02b2c3d479
  a9083afa-1707-4811-a48d-b2ef34cbc85b
```

**Example — list blobs in a vault:**

```bash
vyn relay ls f47ac10b-58cc-4372-a567-0e02b2c3d479
```

```
Blobs:
  ee0a32873a514d1c08f711fd9a7e835ff3243e424f82ece8133b646b0ef19f05  (34 B)
  37a12d33f0ada20f6280ce0b9f6e63cd06a2c0bdc761bc070594303c4e37dc06  (158 B)
```

Blob hashes are SHA-256 of the plaintext. Size is the ciphertext size stored on the relay. All values are opaque to the relay — it never sees plaintext content.
