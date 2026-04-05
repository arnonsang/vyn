# Manifests & Blobs

## Manifest

The manifest is a versioned JSON file that tracks all files in the vault. It is encrypted with the project key before being uploaded to storage.

```json
{
  "version": 1,
  "files": [
    {
      "path": ".env",
      "sha256": "a1b2c3...",
      "size": 512,
      "mode": 33188
    }
  ]
}
```

| Field | Description |
|---|---|
| `path` | Relative path from the project root |
| `sha256` | SHA-256 hash of the plaintext content |
| `size` | Plaintext file size in bytes |
| `mode` | Unix permission bits (preserves `chmod` state) |

The manifest is re-encrypted and re-uploaded on every `vyn push`. On pull, it is downloaded, decrypted in memory, and used to determine which blobs to fetch.

## Blobs

Each file is stored as an encrypted, content-addressed blob:

```
.vyn/blobs/<sha256>.enc
```

The blob filename is the SHA-256 hash of the **plaintext** content. This provides natural deduplication: if two projects track the same file, the blob is only stored once per vault.

**Upload process:**
1. Hash the plaintext content (SHA-256)
2. Check if `.vyn/blobs/<hash>.enc` already exists locally (skip re-encryption)
3. Encrypt with AES-256-GCM using the PK + fresh nonce
4. Upload ciphertext to relay (or memory store)

**Download process:**
1. Decrypt the manifest
2. For each file entry, check if the blob is cached locally
3. Fetch missing blobs from the relay
4. Decrypt each blob in memory
5. Write plaintext to disk at the original path

## Diff engine

When you run `vyn st -v` or `vyn diff`, vyn:

1. **Detects binary files** by scanning the first 1KB for null bytes or non-UTF-8 sequences
2. For **text files**, uses the `similar` crate to produce a unified diff
3. For **binary files**, shows a size change summary

The diff compares the local working file against the baseline recorded in `.vyn/manifest.json` (the state at the last `vyn push` or `vyn pull`).
