# Sharing Keys

Key sharing uses **asymmetric key wrapping**: the sender encrypts the PK with the recipient's public SSH key. Only the recipient's private key can unwrap it.

## Flow: vyn share @bob

```mermaid
sequenceDiagram
    participant Lead as Project Lead (Alice)
    participant GH as GitHub API
    participant CLI as vyn CLI (Alice)
    participant Cloud as vyn Relay
    participant Peer as Teammate (Bob)
    participant OS as Bob's OS Keychain

    Lead->>CLI: vyn share @bob
    CLI->>GH: GET github.com/bob.keys
    GH-->>CLI: Bob's Public SSH Keys (RSA/Ed25519)

    CLI->>CLI: Fetch PK from Keychain
    Note over CLI: Wrap PK with each of Bob's keys (age)

    CLI->>Cloud: Upload Encrypted Invite(s) for Bob

    Note over Peer: Bob runs vyn link <vault_id>
    Peer->>Cloud: GET Invite for Bob
    Cloud-->>Peer: Encrypted Invite Blob

    Note over Peer: Uses ~/.ssh/id_ed25519
    Peer->>Peer: Decrypt Invite → Extract PK
    Peer->>OS: Store PK in Bob's Keychain
```

## Why this is secure

1. **No shared passwords** — you never transmit the PK in plaintext
2. **Identity-bound** — only the holder of the SSH private key can unlock the invite
3. **Relay-blind** — the relay stores only ciphertext and cannot read the PK
4. **Per-key invites** — if Bob has multiple SSH keys on GitHub, one invite is created for each key so any of his machines can link

## Commands

```bash
# Share with a teammate
vyn share @bob

# Share with yourself (for multi-device setup)
vyn share @me

# Accept an invite
vyn link <vault_id>
```

See [vyn share / link](../cli/share-link.md) for full CLI reference.
