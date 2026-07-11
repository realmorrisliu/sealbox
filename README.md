# Sealbox

[![CI](https://github.com/realmorrisliu/sealbox/workflows/CI/badge.svg)](https://github.com/realmorrisliu/sealbox/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)

> A lightweight, self-hosted local secret storage service with client-side encryption

Sealbox is a simple yet secure secret management solution designed for developers and small teams. Built with Rust, it provides envelope encryption, SQLite storage, and a REST API in a single binary with minimal configuration required.

## Features

- 🔐 **Client-side secret encryption** - The CLI encrypts secret values before sending them to the server
- ⏰ **TTL Support** - Automatic expiration with lazy cleanup strategy
- 📦 **Single binary** - No complex setup, just run the executable
- 🗃️ **SQLite storage** - Embedded database, no external dependencies
- 🔑 **Secret versioning** - Keep track of secret history
- 🧱 **Tenant isolation** - Opaque tenant tokens, independent active keys, and tenant-scoped metadata/data APIs
- 🌐 **REST API** - Standard HTTP interface for integration
- 💻 **Full-featured CLI** - Complete command-line interface for key and secret management
- 🔄 **Multiple output formats** - JSON, YAML, and table formats supported
- 🖥️ **Modern Web UI** - React-based web interface with real-time TTL indicators
- 🌍 **Internationalization Ready** - All text and UI elements use English by default

## Quick Start

### Prerequisites

- Rust 1.95+ (for building from source)

### Installation

```bash
# Clone the repository
git clone https://github.com/realmorrisliu/sealbox.git
cd sealbox

# Build the project
cargo build --release
```

### Running the Server

```bash
# Set environment variables
export STORE_PATH=/var/lib/sealbox.db
export AUTH_TOKEN=your-secret-token
# Or use AUTH_TOKEN_FILE=/run/secrets/sealbox_auth_token
export LISTEN_ADDR=127.0.0.1:8080

# Start the server
./target/release/sealbox-server
```

### Setting Up the CLI

```bash
# Initialize configuration
./target/release/sealbox-cli config init

# Generate RSA key pair
./target/release/sealbox-cli key generate

# Register public key with server
./target/release/sealbox-cli key register --url http://localhost:8080 --token your-secret-token
```

### Managing Secrets

#### Using the CLI
```bash
# Store a secret
./target/release/sealbox-cli secret set mypassword "super-secret-value"

# Store a temporary secret (expires in 1 hour)
./target/release/sealbox-cli secret set temp-token "abc123" --ttl 3600

# Retrieve a secret
./target/release/sealbox-cli secret get mypassword

# Generate a strong password locally
./target/release/sealbox-cli password generate --length 32

# Generate an alphanumeric random-id-style password
./target/release/sealbox-cli password generate --alphanumeric --length 40

# Store a username/password credential
./target/release/sealbox-cli credential set db/postgres --username app_user

# Store a credential with a generated password
./target/release/sealbox-cli credential set db/postgres --username app_user --generate-password

# Store a credential password from piped stdin
printf '%s\n' "db-password" | ./target/release/sealbox-cli credential set db/postgres --username app_user

# List credentials, optionally filtering by credential name/key or username
./target/release/sealbox-cli credential list --name db/
./target/release/sealbox-cli credential list --username app
./target/release/sealbox-cli credential list --query postgres

# Export an encrypted archive for backup or migration
./target/release/sealbox-cli secret export backups/sealbox-export.tar.enc

# Import an encrypted archive into the current server
./target/release/sealbox-cli secret import backups/sealbox-export.tar.enc

# List all commands
./target/release/sealbox-cli --help
```

### Multi-Tenant Service Mode

The server root token administers tenants but is not accepted by v2 tenant data
routes. Each tenant receives its own high-entropy API token and registers its
own public key. Tenant tokens are stored only as hashes by the server; the CLI
writes a newly issued plaintext token once to a new mode-`0600` file.

```bash
# Root/admin client. AUTH_TOKEN_FILE is the server root token file.
export SEALBOX_URL=http://127.0.0.1:8080
export SEALBOX_TOKEN_FILE=/run/secrets/sealbox_root_token
sealbox-cli tenant create \
  --display-name "Application A" \
  --token-label initial \
  --token-file /secure/application-a/token

# Tenant data client. The server does not need these key files.
export SEALBOX_API_VERSION=v2
export SEALBOX_TOKEN_FILE=/secure/application-a/token
export SEALBOX_PUBLIC_KEY_FILE=/secure/application-a/public.pem
export SEALBOX_PRIVATE_KEY_FILE=/secure/application-a/private.pem
sealbox-cli key generate
sealbox-cli key register
printf '%s\n' 'secret-value' | sealbox-cli secret set api-key
sealbox-cli secret get api-key
```

Use `sealbox-cli tenant token create`, `list`, and `revoke` to rotate tenant
API access. Suspending a tenant blocks all of its data tokens without deleting
encrypted records. Sealbox does not know or require an upstream application's
user model; that application owns any user-to-tenant mapping.

## Docker Container How-To

The Docker image contains both `sealbox-server` and `sealbox-cli`. Run the server as the default container command, then use short-lived CLI containers on the same network namespace for key setup, secret operations, and archive import/export.

### Build the Image

```bash
docker build -t sealbox:local .
```

### Prepare Local Files

```bash
mkdir -p .sealbox-secrets .sealbox-keys backups
openssl rand -base64 32 > .sealbox-secrets/auth_token

# Bind-mounted Docker secret files must be readable by the non-root
# sealbox user inside the server container. Docker-managed secrets are
# usually mounted this way automatically.
chmod 0444 .sealbox-secrets/auth_token
```

### Start the Server Container

```bash
docker volume create sealbox-data

docker run -d \
  --name sealbox \
  -p 127.0.0.1:8080:8080 \
  -v sealbox-data:/data \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -e AUTH_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  sealbox:local

curl -fsS http://127.0.0.1:8080/healthz/ready
```

`AUTH_TOKEN_FILE` is read by the server as the root bearer-token contents. The server does not need tenant token, public-key, or private-key files.

### Generate and Register Keys

Use the same image as a CLI container. `--network container:sealbox` lets the CLI reach the server at `http://127.0.0.1:8080` without publishing extra ports.

```bash
docker run --rm \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-keys:/keys" \
  sealbox:local \
  sealbox-cli key generate \
    --public-key-path /keys/public_key.pem \
    --private-key-path /keys/private_key.pem

chmod 0400 .sealbox-keys/private_key.pem
chmod 0444 .sealbox-keys/public_key.pem

docker run --rm \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli key register
```

`SEALBOX_TOKEN_FILE` is read by the CLI as the bearer-token contents. `SEALBOX_PUBLIC_KEY_FILE` and `SEALBOX_PRIVATE_KEY_FILE` are paths to mounted PEM key files.

### Store and Retrieve Secrets

Use `-it` for commands that prompt for hidden input. For automation, omit `-t` and pipe the secret or password on stdin.

```bash
docker run --rm -it \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli secret set db/password

docker run --rm \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli secret get db/password
```

To store a searchable username/password pair:

```bash
docker run --rm -it \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli credential set db/postgres --username app_user
```

To generate a password inside the CLI container without touching the server:

```bash
docker run --rm \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  sealbox:local \
  sealbox-cli password generate --alphanumeric --length 40
```

To store a credential with a generated password, omit `-t` unless you also need an interactive terminal for other flags. The generated password is not printed unless `--show-password` is explicitly passed.

```bash
docker run --rm \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli credential set db/postgres \
    --username app_user \
    --generate-password \
    --alphanumeric \
    --length 40
```

Non-interactive credential input works through stdin:

```bash
printf '%s\n' "db-password" | docker run --rm -i \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli credential set db/postgres --username app_user
```

### Export and Import All Secrets

Export produces a versioned encrypted archive. The outer file contains an `envelope_version`; the encrypted tar payload contains `manifest.json` with `format_version` and `secrets.json`.

```bash
docker run --rm \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -v "$PWD/backups:/backups" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli secret export /backups/sealbox-export.tar.enc
```

Import decrypts the archive locally, migrates supported old archive formats, and writes each record through the normal client-side encrypted save path.

```bash
docker run --rm \
  --network container:sealbox \
  --user "$(id -u):$(id -g)" \
  --workdir /tmp \
  -v "$PWD/.sealbox-secrets/auth_token:/run/secrets/sealbox_auth_token:ro" \
  -v "$PWD/.sealbox-keys:/keys:ro" \
  -v "$PWD/backups:/backups:ro" \
  -e SEALBOX_URL=http://127.0.0.1:8080 \
  -e SEALBOX_TOKEN_FILE=/run/secrets/sealbox_auth_token \
  -e SEALBOX_PUBLIC_KEY_FILE=/keys/public_key.pem \
  -e SEALBOX_PRIVATE_KEY_FILE=/keys/private_key.pem \
  sealbox:local \
  sealbox-cli secret import /backups/sealbox-export.tar.enc
```

Keep `.sealbox-keys/private_key.pem` and exported archives protected. Any process with the bearer token and matching private key can retrieve plaintext secrets.

#### Using the Web UI
1. Navigate to the `sealbox-web` directory
2. Install dependencies: `pnpm install`
3. Start the development server: `pnpm run dev`
4. Open http://localhost:3000 in your browser
5. Enter your server URL and AUTH_TOKEN to login
6. Manage secrets through the intuitive web interface
   - Secret creation remains CLI-first because browser-side encryption is not implemented

**Web UI Features:**
- 🔐 Secure token-based authentication
- 📋 Secret list with TTL status indicators
- ⏰ Real-time expiration warnings
- 🗑️ Delete secrets with confirmation
- 📱 Responsive design for mobile devices
- 🌐 CORS support for development
- 🌍 **English-first interface** - All UI elements use clear English text
- 🎨 Modern design with TailwindCSS and shadcn/ui components

## Configuration

### Server Configuration

Configure the server using environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `STORE_PATH` | SQLite database file path | `/var/lib/sealbox.db` |
| `AUTH_TOKEN` | Static bearer token for API authentication | `your-secret-token` |
| `AUTH_TOKEN_FILE` | File containing the bearer token, useful for Docker secrets | `/run/secrets/sealbox_auth_token` |
| `LISTEN_ADDR` | Server listen address and port | `127.0.0.1:8080` |
| `LEGACY_V1_ENABLED` | Expose the root-token v1 compatibility routes | `true` |

### CLI Configuration

The CLI uses TOML configuration files with environment variable overrides:
- Config file: `~/.config/sealbox/config.toml`
- Supports server URL, authentication tokens, key paths, and output preferences

## Security Model

Sealbox implements client-side envelope encryption for CLI writes and reads:

1. **User Key Pair**: Each user generates an RSA key pair locally
2. **Client-Side Encryption**: The CLI fetches the active public key and encrypts the secret locally
3. **Data Keys**: Random AES-256-GCM keys encrypt individual secrets
4. **Envelope Encryption**: Data keys are encrypted with the active RSA public key
5. **Encrypted Storage**: The server stores encrypted data and encrypted data keys in SQLite
6. **Client Decryption**: Only clients with the private key can decrypt retrieved secrets

**Important**: Sealbox is intended as a lightweight local credentials store. If the same Docker runtime has the bearer token and private key, that runtime can retrieve secrets.

For v2, set `SEALBOX_API_VERSION=v2` and use a tenant token with that tenant's
matching key pair. The root token is accepted only by `/v2/admin/*` and legacy
v1 routes; it is rejected by v2 tenant data routes. Set
`LEGACY_V1_ENABLED=false` after all v1 clients have migrated.

Before the first tenant-schema migration of a populated database, startup writes
`<STORE_PATH>.pre-tenant-v2.bak` using SQLite's consistent backup path and
refuses migration if a secret references no master key in the same namespace.
Inspect an existing database without modifying it with:

```bash
sealbox-server migration-report --store-path /var/lib/sealbox/sealbox.db
```

Restore the backup to roll back a failed first migration. Do not attempt to
reverse-migrate a database after v2 tenants have begun writing data.

Credential commands store the username and password together inside the encrypted secret value. The username is also duplicated into plaintext `metadata` so credentials can be listed and searched by username without decrypting every value.

Sealbox keeps the latest 10 versions for each credential key and automatically prunes older credential versions when a new one is saved.

Password generation happens locally in the CLI using the Rust randomness stack already bundled with Sealbox. `password generate` prints generated values; `credential set --generate-password` stores the generated password without printing it unless `--show-password` is passed. Use `--alphanumeric` when you need random-ID-style values without symbols.

Exported archives are encrypted locally: the CLI builds a tar payload in memory, encrypts it with AES-256-GCM, encrypts the archive data key with the configured public key, and writes a versioned envelope so future importers can migrate older archive structures.

## How It Works

```
┌─────────────┐   encrypt locally   ┌──────────────┐    store     ┌──────────────┐
│   Secret    │────────────────────▶│ Encrypted    │─────────────▶│   Server +   │
│ (plaintext) │                     │ Secret +     │              │   SQLite     │
│             │                     │ Encrypted    │              │              │
│             │                     │ Data Key     │              │              │
└─────────────┘                     └──────────────┘              └──────────────┘
```

---

## API Reference

### Tenant Administration (v2 root token)

```text
POST   /v2/admin/tenants
GET    /v2/admin/tenants
GET    /v2/admin/tenants/:tenant_id
POST   /v2/admin/tenants/:tenant_id/suspend
POST   /v2/admin/tenants/:tenant_id/resume
POST   /v2/admin/tenants/:tenant_id/tokens
GET    /v2/admin/tenants/:tenant_id/tokens
DELETE /v2/admin/tenants/:tenant_id/tokens/:token_id
```

Secret and master-key routes under `/v2` mirror the v1 paths but derive tenant
scope exclusively from the bearer token. A tenant id in a request path, query,
or body cannot select another tenant.

All endpoints require `Authorization: Bearer <token>` header.

### Secrets Management
```bash
# List all secrets with metadata
GET /v1/secrets
# Returns: {"secrets": [{"key": "...", "version": 1, "created_at": ..., "updated_at": ..., "expires_at": ...}]}

# Store a secret
PUT /v1/secrets/:key
Content-Type: application/json
{ 
  "encrypted_data": [1, 2, 3],
  "encrypted_data_key": [4, 5, 6],
  "master_key_id": "00000000-0000-0000-0000-000000000000",
  "ttl": 3600
}

# Retrieve a secret (latest version, automatically checks expiration)
GET /v1/secrets/:key

# Retrieve specific version
GET /v1/secrets/:key?version=1

# List retained version metadata
GET /v1/secrets/:key/history

# Delete a secret and all versions
DELETE /v1/secrets/:key

# Delete a specific secret version
DELETE /v1/secrets/:key?version=1
```

### TTL Behavior
- **TTL**: Time-to-live in seconds from creation time
- **Lazy Cleanup**: Expired secrets are deleted when accessed, not immediately when they expire
- **Startup Cleanup**: Server removes expired secrets on startup
- **Manual Cleanup**: Use admin endpoint to batch-remove expired secrets

### Key Management
```bash
# Register public key
POST /v1/master-key
Content-Type: application/json
{ "public_key": "-----BEGIN PUBLIC KEY-----..." }

# List public keys
GET /v1/master-key
# Returns: {"master_keys": [...]}

# Fetch active public key for client-side encryption
GET /v1/master-key/active

# Fetch a specific public key
GET /v1/master-key/by-id/:id

# Fetch encrypted records for client-side data-key rewrap
GET /v1/master-key/by-id/:id/secrets

# Rotate keys with client-side rewrapped data keys
PUT /v1/master-key
```

### Health Check Endpoints
```bash
# Liveness probe (no authentication required)
GET /healthz/live
# Returns: {"result": "Ok", "timestamp": 1640995200}

# Readiness probe (no authentication required)  
GET /healthz/ready
# Returns: {"result": "Ok", "timestamp": 1640995200} if ready
# Returns: 503 status with error details if not ready
```

### Administration
```bash
# Manually clean up all expired secrets
DELETE /v1/admin/cleanup-expired

# Response:
{
  "deleted_count": 15,
  "cleaned_at": 1640995200
}
```

## Development

### Building

```bash
# Build everything
cargo build --release

# Build server only
cargo build --release -p sealbox-server

# Build CLI only
cargo build --release -p sealbox-cli
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Test specific package
cargo test -p sealbox-server
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Security audit
cargo audit
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test`)
6. Run formatting and linting (`cargo fmt && cargo clippy`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

## Roadmap

- [x] **TTL Support** - Automatic expiration with lazy cleanup strategy ✅
- [x] **Web UI for secret management** - React-based web interface ✅
- [x] **CORS Support** - Cross-origin requests for web development ✅
- [x] **Kubernetes Health Checks** - Standard `/healthz/live` and `/healthz/ready` endpoints ✅
- [x] **English-first Internationalization** - All UI and code comments in English ✅
- [ ] **i18n Support** - Multi-language interface with language switching
- [ ] **JWT Authentication** - Replace static token with JWT-based auth
- [ ] **Advanced Secret Operations** - Create, edit, and manage secrets via Web UI
- [ ] **Master Key Management UI** - Complete key management through web interface
- [ ] **Docker and Kubernetes** - Production deployment guides and manifests
- [ ] **Multi-node Replication** - High availability with Raft consensus

## Support

- 🐛 [Issue Tracker](https://github.com/realmorrisliu/sealbox/issues)
- 💬 [Discussions](https://github.com/realmorrisliu/sealbox/discussions)

---

## License

Apache License 2.0
