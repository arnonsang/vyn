# gRPC Protocol

The relay exposes a single gRPC service defined in `proto/vyn.proto`.

## Service definition

```protobuf
service VynRelay {
  rpc Authenticate(AuthRequest) returns (AuthResponse);
  rpc RegisterIdentity(RegisterRequest) returns (RegisterResponse);
  rpc GetManifest(GetManifestRequest) returns (GetManifestResponse);
  rpc PutManifest(PutManifestRequest) returns (PutManifestResponse);
  rpc UploadBlob(stream UploadBlobChunk) returns (UploadBlobResponse);
  rpc DownloadBlob(DownloadBlobRequest) returns (stream DownloadBlobChunk);
  rpc CreateInvite(CreateInviteRequest) returns (CreateInviteResponse);
  rpc GetInvites(GetInvitesRequest) returns (GetInvitesResponse);
  rpc ListVaults(ListVaultsRequest) returns (ListVaultsResponse);
  rpc ListBlobs(ListBlobsRequest) returns (ListBlobsResponse);
}
```

## RPC summary

| RPC | Kind | Description |
|---|---|---|
| `Authenticate` | Unary | SSH challenge-response. Returns a session token. |
| `RegisterIdentity` | Unary | Register a GitHub username + SSH public key on the relay. |
| `GetManifest` | Unary | Download the encrypted manifest for a vault. |
| `PutManifest` | Unary | Upload (replace) the encrypted manifest for a vault. |
| `UploadBlob` | Client streaming | Upload an encrypted blob in chunks. |
| `DownloadBlob` | Server streaming | Download an encrypted blob in chunks. |
| `CreateInvite` | Unary | Upload an age-encrypted invite for a specific user. |
| `GetInvites` | Unary | Fetch all invites for a user/vault pair. |
| `ListVaults` | Unary | List vault IDs accessible to the authenticated user. |
| `ListBlobs` | Unary | List blob hashes and sizes inside a specific vault. |

## Authentication

The relay uses a two-step challenge-response:

1. **RegisterIdentity** — client registers `user_id` + `public_key` + an SSH signature over `"vyn-register:{user_id}:{public_key}"` proving key ownership
2. **Authenticate** — client signs a relay-issued nonce with their private key; relay returns a bearer token used in subsequent requests

## Streaming blobs

`UploadBlob` and `DownloadBlob` use gRPC streaming to handle large blobs efficiently without buffering the entire ciphertext in memory.

```protobuf
message UploadBlobChunk {
  string hash = 1;   // SHA-256 of the plaintext (blob identifier)
  bytes  chunk = 2;  // ciphertext chunk
}

message DownloadBlobChunk {
  bytes chunk = 1;   // ciphertext chunk
}
```

## Proto source

The canonical proto source is [`proto/vyn.proto`](https://github.com/arnonsang/vyn/blob/main/proto/vyn.proto). It is bundled into `crates/vyn-relay/proto/` for publishing.
