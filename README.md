# Sealbox

A Simple Secret Storage Service – self-hosted and developer-friendly.

Sealbox is a lightweight, single-node secret storage service designed for developers and small teams. It supports envelope encryption, embedded storage via SQLite, and a simple REST API to manage secrets securely in local or edge environments.

---

## Why Sealbox?

Most secret management solutions like HashiCorp Vault, AWS Secrets Manager, or GCP Secret Manager are powerful—but also complex, over-engineered, and deeply cloud-integrated. They often assume enterprise-scale deployments, dynamic secret provisioning, complex ACLs, and heavy agent-based integrations.

Sealbox is different.

Sealbox is built for developers and small teams who value:
- **Simplicity**: No servers to cluster (unless you want to), no plugins to configure, no cloud dependencies.
- **Security by default**: AES-GCM envelope encryption, zero plaintext storage, JWT-based auth, namespace isolation.
- **Single-binary, opinionated design**: Embedded SQLite, stateless API, and minimal configuration—all in one self-contained binary.
- **Predictability**: Instead of flexible-but-complex policies, Sealbox favors convention: one secret = one key, no magic.
- **Designed for CI, containers, and local-first environments**: Works equally well in Docker, bare-metal, or Kubernetes.

Sealbox doesn’t aim to replace Vault. It aims to be the 90% simpler alternative when you don’t need dynamic database credentials or secret leasing—but still want safe, verifiable secret storage.

## When Not to Use Sealbox?
- You need dynamic database credentials (use HashiCorp Vault).
- You require fine-grained multi-tenant ACLs (future roadmap).
- Your secrets must sync across regions (consider cloud-native solutions).

---

## Features

### MVP 0.1.0
- Envelope encryption
- Static token auth
- SQLite storage
- PUT/GET/DELETE HTTP API
- TTL field support (no GC)
- REST API only (no CLI)

### v1.0.0
- Sealbox CLI
- JWT authentication (with replay protection)
- Secret versioning
- Automatic TTL expiration cleanup
- Raft replication for multi-replica SQLite
- Docker Compose support
- Helm Chart support (Kubernetes)

### v1.1.0
- Web UI
- Access audit logging
- CLI secret decryption cache
- Metadata query API

### Future
- External KMS support (AWS, Vault)
- TPM/YubiKey hardware key support
- Multi-tenant ACL
- Pluggable crypto backend
- CLI auto-login via OAuth2 Device Code Flow
- Additional authentication strategies

---

## Getting Started

```bash
# Build Sealbox (Rust required)
cargo build --release

# Run Sealbox
STORE_PATH=/var/lib/sealbox.db \
MASTER_KEY_PATH=/var/lib/sealbox.key \
AUTH_TOKEN=secrettoken123 \
LISTEN_ADDR=127.0.0.1:8080 \
./target/release/sealbox
```

On first startup, Sealbox will:
- Generate a random 256-bit master key (if not found at `MASTER_KEY_PATH`)
- Save the base64-encoded master key to the configured file
- Print a one-time recovery mnemonic for backup and recovery

The server will then enter active state and be ready to serve requests.

---

## REST API

| Method | Path               | Description                     |
|--------|--------------------|---------------------------------|
| PUT    | /v1/secrets/:key   | Encrypt and store secret        |
| GET    | /v1/secrets/:key   | Decrypt and return secret       |
| DELETE | /v1/secrets/:key   | Delete a stored secret          |

Query parameters:
- `namespace`: logical group for secrets (if applicable)
- `key`: the name of the secret

Headers:
- `Authorization: Bearer <JWT>`

Note: The `version` field is automatically incremented on each `PUT`, but not exposed via the API in v1.0. Historical versions are stored internally for future use (e.g., audit, rollback) but not exposed to clients.

---

## Authentication

Sealbox supports two authentication mechanisms:

### 1. Kubernetes Auth (Cloud-native)
- For services deployed in Kubernetes, Sealbox authenticates via Pod ServiceAccount tokens
- The client (CLI or SDK) reads the token from the Pod’s filesystem and authenticates via `POST /api/auth/k8s`
- The server verifies the token against the Kubernetes cluster CA and issues a signed JWT

### 2. Device Login (Local / Server / CI)
- For developer machines, Docker Compose, CI pipelines, or bare-metal Linux
- Run `sealbox login --device`
- Sealbox will show a one-time verification code and URL
- Open the URL in a browser, authenticate, and enter the code
- Once verified, the CLI receives a signed JWT and stores it locally

This covers both interactive and automated environments in a secure and unified way.

#### Auth Token Format

Sealbox issues JWT tokens signed using RSA256. The public key is exposed at:
```
GET /api/auth/public.pem
```

Claims may include:
- `sub`: subject (user or service identity)
- `exp`: expiry timestamp (e.g. 24h)
- `aud`: audience validation (e.g. `sealbox`)
- `scope`: permissions (reserved for future use)
- `jti`: unique token ID (for replay protection)

#### JWT Replay Protection (v1.0)

To prevent replay attacks, Sealbox will implement nonce-based token tracking:
- Each JWT will include a `jti` (JWT ID) claim that uniquely identifies the token
- The server will maintain an in-memory or persistent store of recently seen `jti` values
- Each authenticated request will check whether the `jti` has been used before
- Expired or reused `jti` values will be rejected

---

## Storage Design

Sealbox uses a single `secrets` table in SQLite with the following fields:
- `id` (primary key)
- `namespace` (TEXT)
- `key` (TEXT)
- `version` (INTEGER)
- `encrypted_value` (BLOB)
- `created_at` (TIMESTAMP)
- `expires_at` (TIMESTAMP)
- `created_by` (TEXT, optional)

This schema supports:
- Versioning (by incrementing `version` per insert)
- TTL (`expires_at`)
- Auditing (`created_by`, `created_at`)

**Note**: While `version` is stored internally, the external API only supports retrieving the latest version for simplicity. This preserves the 1-key-1-secret UX while keeping future rollback or auditing capabilities.

### Note on Raft and SQLite

In v1.0, Sealbox will support multi-replica deployments using Raft. Since SQLite does not support concurrent writes, Sealbox will apply the Raft consensus log and WAL replay pattern: only the leader will accept writes, replicate them via Raft log, and followers will apply the same changes deterministically to their local SQLite copies.

---

## Configuration

Sealbox is configured via environment variables:

```env
STORE_PATH=/var/lib/sealbox.db
MASTER_KEY_PATH=/var/lib/sealbox.key
LISTEN_ADDR=127.0.0.1:8080
AUTH_TOKEN=secrettoken123
```

---

## Example (curl)

```bash
curl -X PUT http://localhost:8080/v1/secrets/db-password \
     -H "Authorization: Bearer secrettoken123" \
     -d 'supersecret'

curl -X GET http://localhost:8080/v1/secrets/db-password \
     -H "Authorization: Bearer secrettoken123"

curl -X DELETE http://localhost:8080/v1/secrets/db-password \
     -H "Authorization: Bearer secrettoken123"
```

---

## License

Apache License 2.0
