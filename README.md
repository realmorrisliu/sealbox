# Sealbox

[![CI](https://github.com/realmorrisliu/sealbox/workflows/CI/badge.svg)](https://github.com/realmorrisliu/sealbox/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

A Simple Secret Storage Service – self-hosted and developer-friendly.

Sealbox is a lightweight, single-node secret storage service designed for developers and small teams. It supports envelope encryption, embedded storage via SQLite, and a simple REST API to manage secrets securely in local or edge environments.

---

## Why Sealbox?

Most secret management solutions like HashiCorp Vault, AWS Secrets Manager, or GCP Secret Manager are powerful—but also complex, over-engineered, and deeply cloud-integrated. They often assume enterprise-scale deployments, dynamic secret provisioning, complex ACLs, and heavy agent-based integrations.

Sealbox is different.

Sealbox is built for developers and small teams who value:
- **Simplicity**: No servers to cluster (unless you want to), no plugins to configure, no cloud dependencies.
- **Security by default**: AES-GCM envelope encryption, zero plaintext storage, and simple token-based auth.
- **Single-binary, opinionated design**: Embedded SQLite, stateless API, and minimal configuration—all in one self-contained binary.
- **Predictability**: Instead of flexible-but-complex policies, Sealbox favors convention: one secret = one key, with multiple versions.
- **Designed for CI, containers, and local-first environments**: Works equally well in Docker, bare-metal, or Kubernetes.

Sealbox doesn’t aim to replace Vault. It aims to be the 90% simpler alternative when you don’t need dynamic database credentials or secret leasing—but still want safe, verifiable secret storage.

## When Not to Use Sealbox?
- You need dynamic database credentials (use HashiCorp Vault).
- You require fine-grained multi-tenant ACLs (future roadmap).
- Your secrets must sync across regions (consider cloud-native solutions).

---

## Features

### MVP 0.1.0
- [x] Envelope encryption
- [x] Static token auth
- [x] SQLite storage
- [x] Secret versioning
- [x] PUT/GET/DELETE HTTP API
- [x] TTL field support (no GC)
- [x] REST API
- [x] Sealbox CLI to create master key

### v1.0.0
- [ ] Sealbox CLI
- [ ] JWT authentication (with replay protection)
- [ ] Automatic TTL expiration cleanup
- [ ] Raft replication for multi-replica SQLite
- [ ] Docker Compose support
- [ ] Helm Chart support (Kubernetes)

### v1.1.0
- [ ] Web UI
- [ ] Access audit logging
- [ ] CLI secret decryption cache
- [ ] Metadata query API

### Future
- [ ] External KMS support (AWS, Vault)
- [ ] TPM/YubiKey hardware key support
- [ ] Multi-tenant ACL
- [ ] Pluggable crypto backend
- [ ] CLI auto-login via OAuth2 Device Code Flow
- [ ] Additional authentication strategies

---

## Development

### Running Tests

Sealbox includes comprehensive unit tests covering encryption, storage, and API functionality:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests for specific package
cargo test -p sealbox-server
cargo test -p sealbox-cli

# Run specific test
cargo test test_encrypt_decrypt
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for security vulnerabilities
cargo audit
```

---

## Getting Started

### 1. Run Sealbox Server
```bash
# Build Sealbox (Rust required)
cargo build --release

# Run Sealbox Server (set required environment variables)
STORE_PATH=/var/lib/sealbox.db \
AUTH_TOKEN=secrettoken123 \
LISTEN_ADDR=127.0.0.1:8080 \
./target/release/sealbox-server
```

**Environment variables:**
- `STORE_PATH`: SQLite database file path
- `AUTH_TOKEN`: Static bearer token for API authentication (remember this for CLI commands)
- `LISTEN_ADDR`: Server listen address and port

The server will start and be ready to serve requests.

### 2. Create and Register a Master Key
Sealbox uses an end-to-end encryption model where the client generates and holds the private key. You must register your public key with the server before you can store secrets.

Use the `sealbox-cli` to generate a new key pair and register the public key with the server in one step:

```bash
# Build the CLI
cargo build --release -p sealbox-cli

# Create a new master key (generates key pair if not found, then registers public key)
# Note: --token is required and must match the AUTH_TOKEN from server configuration
./target/release/sealbox-cli master-key create \
    --token secrettoken123 \
    --url http://localhost:8080 \
    --public-key-path my_public_key.pem \
    --private-key-path my_private_key.pem
```

**Command parameters:**
- `--token`: Required. Authentication token that must match the server's `AUTH_TOKEN` environment variable
- `--url`: Server URL (default: http://127.0.0.1:8080)
- `--public-key-path`: Path for public key file (default: public_key.pem)
- `--private-key-path`: Path for private key file (default: private_key.pem)

This command will:
1. Check for `my_public_key.pem` and `my_private_key.pem`.
2. If not found, generate a new RSA key pair and save them to these files.
3. Register the public key with the running `sealbox-server`.

---

## REST API

All endpoints are protected and require an `Authorization: Bearer <token>` header.

### Secrets
| Method | Path               | Description                     |
|--------|--------------------|---------------------------------|
| `PUT`    | `/v1/secrets/:key`   | Creates a new version of a secret. |
| `GET`    | `/v1/secrets/:key`   | Retrieves a secret. Defaults to the latest version. |
| `DELETE` | `/v1/secrets/:key`   | Deletes a specific version of a secret. |

### Master Keys
| Method | Path               | Description                     |
|--------|--------------------|---------------------------------|
| `POST`   | `/v1/master-key`     | Registers a new public key for encrypting secrets. |
| `GET`    | `/v1/master-key`     | Lists all registered public keys. |
| `PUT`    | `/v1/master-key`     | Rotates secrets from an old key to a new one. |

---

## Authentication

For the current version, Sealbox uses a simple static bearer token for authentication. All API requests must include an `Authorization` header with the token specified in the `AUTH_TOKEN` environment variable.

**Example Header:**
`Authorization: Bearer secrettoken123`

Future versions will introduce more advanced JWT-based authentication mechanisms as outlined in the roadmap.

---

## Configuration

Sealbox is configured via environment variables:

```env
# The path to the SQLite database file.
STORE_PATH=/var/lib/sealbox.db

# The address and port for the server to listen on.
LISTEN_ADDR=127.0.0.1:8080

# The static bearer token for API authentication.
AUTH_TOKEN=secrettoken123
```

---

## Example (curl)

### 1. Create a new secret version
This creates a new version of the secret `db-password`. The optional `ttl` is in seconds.
```bash
curl -X PUT http://localhost:8080/v1/secrets/db-password \
     -H "Authorization: Bearer secrettoken123" \
     -H "Content-Type: application/json" \
     -d '{ "secret": "supersecret", "ttl": 3600 }'
```

### 2. Get the latest version of a secret
```bash
curl -X GET http://localhost:8080/v1/secrets/db-password \
     -H "Authorization: Bearer secrettoken123"
```

### 3. Get a specific version of a secret
```bash
curl -X GET "http://localhost:8080/v1/secrets/db-password?version=1" \
     -H "Authorization: Bearer secrettoken123"
```

### 4. Delete a specific version of a secret
```bash
curl -X DELETE "http://localhost:8080/v1/secrets/db-password?version=1" \
     -H "Authorization: Bearer secrettoken123"
```

---

## Storage Design

Sealbox uses end-to-end encryption (E2EE) by default: secrets are always encrypted with a user-held private key model. The server never has access to the keys required to decrypt user data.

### End-to-End Encryption (E2EE, User-Held Private Key)

**How it works:**
- Each user generates a key pair (public/private).
- For each secret, a random Data Key is generated.
- The secret value is encrypted with the Data Key (`encrypted_data`).
- The Data Key is encrypted with the user’s public key (`encrypted_data_key`).
- Only the user, holding the private key, can decrypt the Data Key and thus the secret.
- The server only stores encrypted data and public keys.

---

## License

Apache License 2.0
