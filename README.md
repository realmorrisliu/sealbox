# Sealbox

A Simple Secret Storage Service – self-hosted and developer-friendly.

Sealbox is a lightweight, single-node secret storage service designed for developers and small teams. It supports envelope encryption, embedded storage via SQLite, and a simple REST API to manage secrets securely in local or edge environments.

## Features

- AES-256-GCM envelope encryption with per-secret data keys
- Embedded SQLite persistence
- REST API for storing, retrieving, and deleting secrets
- TTL support for optional expiration
- Static token authentication (JWT coming in v1.0.0)
- Lightweight CLI tool for usage automation
- Designed for easy deployment and strong local security

## Getting Started

```bash
# Build Sealbox (Rust required)
cargo build --release

# Run Sealbox
env MASTER_KEY=$(openssl rand -base64 32) \
    AUTH_TOKEN=secrettoken123 \
    ./target/release/sealbox
```

## REST API

| Method | Path               | Description                     |
|--------|--------------------|---------------------------------|
| PUT    | /v1/secrets/:key   | Encrypt and store secret        |
| GET    | /v1/secrets/:key   | Decrypt and return secret       |
| DELETE | /v1/secrets/:key   | Delete a stored secret          |

## Configuration

Sealbox can be configured using environment variables or a config file (TOML):

```env
MASTER_KEY="...base64..."
AUTH_TOKEN="secrettoken123"
STORE_PATH="/var/lib/sealbox.db"
LISTEN_ADDR=":8080"
```

## CLI Usage

```bash
sealbox put db-password supersecret
sealbox get db-password
sealbox delete db-password
```

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

## Roadmap

### MVP 0.1.0

- Envelope encryption
- Static token auth
- SQLite storage
- PUT/GET/DELETE API
- TTL field support (no GC)
- CLI tool (basic)

### v1.0.0

- Secret versioning
- Automatic TTL expiration cleanup
- JWT auth with public key verification
- Access audit logging
- CLI secret decryption cache
- Web UI
- Docker deployment
- Cluster deployment with Raft consensus

### Future

- External KMS support (AWS, Vault)
- TPM/YubiKey hardware key support
- Multi-tenant ACL
- Metadata querying
- Pluggable crypto backend

## License

Apache License 2.0
