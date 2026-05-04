# vyn push / pull

## vyn push

Encrypt local tracked files and upload to the configured storage provider.

```bash
vyn push
```

**What it does:**

1. Reads `.vyn/config.toml` for vault ID and storage provider
2. Loads the project key from the OS keychain
3. For each tracked file: hashes plaintext, checks if `.vyn/blobs/{hash}.enc` is already cached (skips re-encryption if so), encrypts with AES-256-GCM, uploads ciphertext blob
4. Encrypts the manifest with the project key and uploads it
5. Writes a local history entry

---

## vyn pull

Download encrypted state from the storage provider and restore files locally.

```bash
vyn pull
```

**What it does:**

1. Loads the project key from the OS keychain
2. Downloads the encrypted manifest and decrypts it in memory
3. For each blob: downloads encrypted ciphertext, caches to `.vyn/blobs/{hash}.enc`, decrypts in memory, writes plaintext to the original path
4. Updates `.vyn/manifest.json` and writes a history entry

## Notes

- Both commands require a configured storage provider (`vyn config`) and authenticated identity (`vyn auth`) when using relay storage
- After `vyn auth`, subsequent push/pull reuse the cached session token in `.vyn/session.token` - no SSH signing on every command
- If the session token is expired (24h TTL), the command prints a message and asks you to run `vyn auth` again
- Pull overwrites local files with the remote baseline; run `vyn st` first to check for local changes
- Encrypted blobs are cached locally in `.vyn/blobs/`; only new or changed blobs are downloaded on subsequent pulls
- If `.vyn/config.toml` is absent, both commands fall back to reading `vyn.toml` in the project root; makes `vyn pull` work in a freshly-cloned repo (e.g. CI) without extra setup
