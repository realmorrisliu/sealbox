# Sealbox

[![CI](https://github.com/realmorrisliu/sealbox/workflows/CI/badge.svg)](https://github.com/realmorrisliu/sealbox/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

> A lightweight, self-hosted secret storage service with end-to-end encryption

Sealbox is a simple yet secure secret management solution designed for developers and small teams. Built with Rust, it provides envelope encryption, SQLite storage, and a REST API in a single binary with minimal configuration required.

## Features

- 🔐 **Server-side encryption** - Secrets are encrypted using envelope encryption on the server
- ⏰ **TTL Support** - Automatic expiration with lazy cleanup strategy
- 📦 **Single binary** - No complex setup, just run the executable
- 🗃️ **SQLite storage** - Embedded database, no external dependencies
- 🔑 **Secret versioning** - Keep track of secret history
- 🌐 **REST API** - Standard HTTP interface for integration
- 💻 **Full-featured CLI** - Complete command-line interface for key and secret management
- 🔄 **Multiple output formats** - JSON, YAML, and table formats supported
- 🖥️ **Modern Web UI** - React-based web interface with real-time TTL indicators
- 🌍 **Internationalization Ready** - All text and UI elements use English by default

## Quick Start

### Prerequisites

- Rust 1.85+ (for building from source)

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

#### Using the CLI (Optimized for Secret Consumption)
```bash
# Store a secret
./target/release/sealbox-cli secret set mypassword "super-secret-value"

# Store a temporary secret (expires in 1 hour)
./target/release/sealbox-cli secret set temp-token "abc123" --ttl 3600

# Retrieve a secret
./target/release/sealbox-cli secret get mypassword

# Export secrets as environment variables (perfect for CI/CD)
./target/release/sealbox-cli secret export --format env --prefix MY_APP

# Export specific secrets to shell format
./target/release/sealbox-cli secret export --keys "db_*" --format shell --prefix PROD > env.sh

# Import secrets from file
./target/release/sealbox-cli secret import secrets.json

# List all commands
./target/release/sealbox-cli --help
```

#### Using the Web UI (Optimized for Secret Management)
1. Navigate to the `sealbox-web` directory
2. Install dependencies: `pnpm install`
3. Start the development server: `pnpm run dev`
4. Open http://localhost:3000 in your browser
5. Enter your server URL and AUTH_TOKEN to login
6. Manage secrets through the intuitive web interface

**Web UI Features (Management Focus):**
- 🔐 Secure token-based authentication
- 📋 Visual secret overview with search and filtering
- ⏰ Real-time TTL monitoring with expiration warnings
- ➕ Create and delete secrets with visual confirmation
- 🗂️ Batch operations for multiple secrets
- 📊 Secret statistics and usage metrics
- 📱 Responsive design for mobile devices
- 🌍 **4-language support** - English, Chinese, Japanese, German
- 🎨 Modern design with TailwindCSS and shadcn/ui components
- 🌐 CORS support for development

## CLI vs Web UI: Designed for Different Use Cases

Sealbox follows a **separation of concerns** philosophy between its CLI and Web UI:

### 🖥️ CLI: Secret Consumption (Automation & CI/CD)
**Perfect for:**
- Retrieving secrets in scripts and CI/CD pipelines
- Exporting secrets as environment variables
- Importing secrets from configuration files
- One-time secret creation in development
- Key pair generation and registration

**Key Commands:**
```bash
# Generate and register keys (one-time setup)
sealbox-cli key generate && sealbox-cli key register

# Retrieve secrets for use in applications
sealbox-cli secret get database_password

# Export all secrets for CI/CD
sealbox-cli secret export --format shell --prefix PROD

# Import secrets from deployment configs
sealbox-cli secret import config.json
```

### 🌐 Web UI: Secret Management (Visual Administration)
**Perfect for:**
- Visual overview of all secrets and their status
- Creating, editing, and organizing secrets
- Monitoring TTL expiration status
- Managing batch operations
- Team collaboration and secret lifecycle management

**Key Features:**
- 📊 Dashboard with secret statistics
- 🔍 Search and filter capabilities
- ⏰ TTL countdown and expiration alerts
- 🗂️ Batch create/delete operations
- 👥 Multi-language support for teams

This design ensures that **CLI excels at automation** while **Web UI excels at management**, giving you the right tool for each task.

## Configuration

### Server Configuration

Configure the server using environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `STORE_PATH` | SQLite database file path | `/var/lib/sealbox.db` |
| `AUTH_TOKEN` | Static bearer token for API authentication | `your-secret-token` |
| `LISTEN_ADDR` | Server listen address and port | `127.0.0.1:8080` |

### CLI Configuration

The CLI uses TOML configuration files with environment variable overrides:
- Config file: `~/.config/sealbox/config.toml`
- Supports server URL, authentication tokens, key paths, and output preferences

## Security Model

Sealbox implements server-side envelope encryption with client-side decryption:

1. **User Key Pair**: Each user generates an RSA key pair locally
2. **Client Sends Plaintext**: CLI sends secrets as plaintext to the server over HTTPS
3. **Server-Side Encryption**: Server encrypts secrets using envelope encryption
4. **Data Keys**: Random AES-256-GCM keys encrypt individual secrets
5. **Envelope Encryption**: Data keys are encrypted with the user's RSA public key
6. **Client Decryption**: Only clients with the private key can decrypt retrieved secrets

**Important**: While secrets are sent as plaintext to the server, only the user with the corresponding private key can decrypt stored data.

## How It Works

```
┌─────────────┐    send       ┌──────────────┐    encrypt   ┌──────────────┐
│   Secret    │──────────────▶│    Server    │─────────────▶│ Encrypted    │
│ (plaintext) │               │              │              │ Secret +     │
│             │               │              │              │ Encrypted    │
│             │               │              │              │ Data Key     │
└─────────────┘               └──────────────┘              └──────────────┘
```

---

## API Reference

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
  "secret": "your-secret-value",
  "ttl": 3600  # Optional: expires in 3600 seconds (1 hour)
}

# Retrieve a secret (latest version, automatically checks expiration)
GET /v1/secrets/:key

# Retrieve specific version
GET /v1/secrets/:key?version=1

# Delete a secret version
DELETE /v1/secrets/:key?version=1
```

### TTL Behavior
- **TTL**: Time-to-live in seconds from creation time
- **Lazy Cleanup**: Expired secrets are deleted when accessed, not immediately when they expire
- **Startup Cleanup**: Server removes expired secrets on startup
- **Manual Cleanup**: Use admin endpoint to batch-remove expired secrets

### Client Management (by client)
```bash
# Register client (device)
POST /v1/clients
Content-Type: application/json
{ "name": "my-laptop", "public_key": "-----BEGIN RSA PUBLIC KEY-----..." }

# List clients
GET /v1/clients

# Update status
PUT /v1/clients/{client_id}/status

# Rename/update description
PUT /v1/clients/{client_id}/name
```

### Secret Permissions
```bash
# View permissions
GET /v1/secrets/{key}/permissions

# Revoke client permission
DELETE /v1/secrets/{key}/permissions/{client_id}

# Update client's encrypted DataKey (client-side rotation)
PUT /v1/secrets/{key}/permissions/{client_id}/data-key
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
- [ ] **Client Key Management UI** - Complete key management through web interface
- [ ] **Docker and Kubernetes** - Production deployment guides and manifests
- [ ] **Multi-node Replication** - High availability with Raft consensus

## Support

- 🐛 [Issue Tracker](https://github.com/realmorrisliu/sealbox/issues)
- 💬 [Discussions](https://github.com/realmorrisliu/sealbox/discussions)

---

## License

Apache License 2.0
