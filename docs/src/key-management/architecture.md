# Key Architecture

vyn uses a two-layer key model.

## Layer 1: Project Key (symmetric)

The Project Key (PK) is a 256-bit AES key generated with `ring::rand::SecureRandom` when you run `vyn init`. It:

- **Never leaves the machine in plaintext** — it is stored only in the OS keychain
- Is used to encrypt every file blob and the manifest before any network transfer
- Is identified in the keychain by `(service=vyn, account=<vault_id>)`

```mermaid
sequenceDiagram
    participant User as Project Lead
    participant CLI as vyn CLI
    participant OS as OS Keychain
    participant Cloud as vyn Relay / S3

    User->>CLI: vyn init "my-project"
    Note over CLI: Generate 256-bit Random Key (PK)
    CLI->>OS: Store PK (Service: vyn, Account: vault_id)
    Note over OS: Key is now in Secure Enclave
    CLI->>CLI: Create Manifest (v1)
    CLI->>Cloud: Upload Manifest + Project Metadata
    CLI-->>User: Project Initialized
```

**Key property:** The relay never sees the PK. It only knows that a project with a given `vault_id` exists.

## Layer 2: SSH key wrapping (asymmetric)

To share the vault, the PK is wrapped (encrypted) with the recipient's SSH public key using [age](https://age-encryption.org). The wrapped invite is a `.age` file that can only be decrypted by the holder of the matching SSH private key.

Supported SSH key types: **RSA** and **Ed25519**.

## Multi-device sync

```mermaid
sequenceDiagram
    participant PC1 as Machine 1 (Mac)
    participant GH as GitHub
    participant Cloud as vyn Relay
    participant PC2 as Machine 2 (Linux)

    Note over PC1: vyn init
    PC1->>PC1: Generate PK & Store in Mac Keychain

    Note over PC1: vyn share @me
    PC1->>GH: Get ALL Public Keys for @me
    GH-->>PC1: [Key_Mac, Key_Linux]

    PC1->>PC1: Wrap PK with Key_Linux
    PC1->>Cloud: Upload Encrypted Invite for Key_Linux

    Note over PC2: vyn link <vault_id>
    PC2->>Cloud: GET Invites for @me
    PC2->>PC2: Decrypt with Linux Private Key
    PC2->>PC2: Store PK in Linux Keychain
```
