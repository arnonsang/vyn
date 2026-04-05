# Key Rotation

## vyn rotate

Rotate the project key. This generates a new AES-256-GCM key and re-encrypts all remote state.

```bash
vyn rotate
```

## What it does

1. Generates a new 256-bit AES project key via `ring::rand::SecureRandom`
2. Downloads all currently uploaded blobs
3. Re-encrypts each blob with the new key + fresh nonces
4. Re-uploads all blobs and re-encrypts the manifest
5. Updates the OS keychain entry with the new key
6. Rebuilds invite files for known teammates (re-wraps new key with their SSH public keys)
7. Writes a history entry

## When to rotate

- A team member leaves and you want to revoke their access
- You suspect the project key has been compromised
- Routine periodic rotation policy

## Revoking a teammate

Re-key rotation alone does not immediately revoke a teammate who already has the old PK cached in their keychain. After rotating:

1. Do **not** run `vyn share @formercolleague` with the new key
2. The former teammate's keychain still holds the old PK, but their future pushes/pulls will fail because the relay now has state encrypted with the new PK
3. For complete revocation, ensure the relay purges old blobs (contact your relay admin or delete the vault data directory and re-push)
