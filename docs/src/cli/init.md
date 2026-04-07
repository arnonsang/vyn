# vyn init

Initialize a new vault in the current directory.

```bash
vyn init [name]
```

| Argument | Description |
|---|---|
| `name` | Optional project name. Defaults to the current directory name. |

## What it does

1. Fails with an error if `.vyn/config.toml` already exists
2. Creates `.vyn/` and `.vyn/blobs/`
3. Generates a random vault UUID and 256-bit AES-256-GCM project key
4. Stores the project key in the OS keychain under `(service=vyn, account=<vault_id>)`
5. Writes `.vyn/manifest.json` (initial empty file index) and `.vyn/config.toml`
6. Writes `vyn.toml` in the project root with `vault_id` (non-secret — commit this to Git)
7. Adds `.vyn/` to `.gitignore` (creates the file if absent)
8. Copies `.vynignore.example` to `.vynignore` if an example is present

## Example

```bash
$ vyn init backend-config
✓ Vault initialized
  vault_id:    f47ac10b-58cc-4372-a567-0e02b2c3d479
  project:     backend-config
  storage:     unconfigured (run `vyn config` to set up storage)
```

## Next steps

After `vyn init`, run [`vyn config`](./config.md) to configure a storage provider, then [`vyn auth`](./auth.md) to register your identity.
