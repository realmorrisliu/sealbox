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

#### Using the CLI
```bash
# Store a secret
./target/release/sealbox-cli secret set mypassword "super-secret-value"

# Store a temporary secret (expires in 1 hour)
./target/release/sealbox-cli secret set temp-token "abc123" --ttl 3600

# Retrieve a secret
./target/release/sealbox-cli secret get mypassword

# List all commands
./target/release/sealbox-cli --help
```

#### Using the Web UI
1. Navigate to the `sealbox-web` directory
2. Install dependencies: `pnpm install`
3. Start the development server: `pnpm run dev`
4. Open http://localhost:3000 in your browser
5. Enter your server URL and AUTH_TOKEN to login
6. Manage secrets through the intuitive web interface

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

### Key Management
```bash
# Register public key
POST /v1/master-key
Content-Type: application/json
{ "public_key": "-----BEGIN PUBLIC KEY-----..." }

# List public keys
GET /v1/master-key

# Rotate keys
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
