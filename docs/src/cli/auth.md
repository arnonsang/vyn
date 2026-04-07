# vyn auth

Authenticate your local identity using GitHub OAuth and a local SSH key.

```bash
vyn auth
```

## 3-step flow

### Step 1: GitHub OAuth Device Flow

vyn opens `https://github.com/login/device` in your browser and displays a one-time code in the terminal. Confirm the code on GitHub. No username or password entry is required; GitHub verifies your identity.

### Step 2: SSH key detection

vyn automatically finds `~/.ssh/id_ed25519` or `~/.ssh/id_rsa`. If neither is found, it prints an error with instructions.

### Step 3: SSH challenge-response

vyn fetches your registered public keys from `https://github.com/<username>.keys` and runs a local `ssh-keygen` challenge-response to prove you hold the matching private key.

Your local key **must** be listed on GitHub because teammates use it to encrypt vault invites for you. If the key is not on GitHub yet, `vyn auth` prints the key and tells you where to add it.

### With relay storage

If your config points to a relay (`storage_provider = "relay"`), `vyn auth` also registers your GitHub username and SSH public key on the relay. The relay uses this registration to authenticate subsequent push/pull requests.

## Output

On success, writes two identity files:

- **`.vyn/identity.toml`** — local vault identity (current directory)
- **`~/.vyn/identity.toml`** — global identity used by `vyn clone` when starting fresh in a new directory

```toml
github_username = "your-handle"
ssh_private_key = "/home/you/.ssh/id_ed25519"
ssh_public_key  = "/home/you/.ssh/id_ed25519.pub"
```

## Environment variables

| Variable | Description |
|---|---|
| `VYN_GITHUB_CLIENT_ID` | Override the built-in OAuth `client_id` (advanced) |
| `VYN_SKIP_GITHUB_VERIFY=1` | Skip GitHub identity and SSH verification (offline CI only, debug builds) |
