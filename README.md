# Sealbox

[![CI](https://github.com/realmorrisliu/sealbox/workflows/CI/badge.svg)](https://github.com/realmorrisliu/sealbox/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

> A lightweight, self-hosted secret storage service with end-to-end encryption

Sealbox is a simple yet secure secret management solution designed for developers and small teams. Built with Rust, it provides envelope encryption, SQLite storage, and a REST API in a single binary with minimal configuration required.

## Features

- 🔐 **End-to-end encryption** - Your secrets are encrypted locally before reaching the server
- 📦 **Single binary** - No complex setup, just run the executable
- 🗃️ **SQLite storage** - Embedded database, no external dependencies
- 🔑 **Secret versioning** - Keep track of secret history
- 🌐 **REST API** - Standard HTTP interface for integration
- 💻 **Full-featured CLI** - Complete command-line interface for key and secret management
- 🔄 **Multiple output formats** - JSON, YAML, and table formats supported

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

```bash
# Store a secret
./target/release/sealbox-cli secret set mypassword "super-secret-value"

# Retrieve a secret
./target/release/sealbox-cli secret get mypassword

# List all commands
./target/release/sealbox-cli --help
```

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

Sealbox implements end-to-end encryption using envelope encryption:

1. **User Key Pair**: Each user generates an RSA key pair locally
2. **Data Keys**: Random AES keys encrypt individual secrets
3. **Envelope Encryption**: Data keys are encrypted with the user's public key
4. **Zero Knowledge**: The server never has access to decrypt user secrets

## How It Works

```
┌─────────────┐    encrypt    ┌──────────────┐    send     ┌──────────────┐
│   Secret    │──────────────▶│ Encrypted    │────────────▶│    Server    │
│             │               │ Secret +     │             │              │
│             │               │ Encrypted    │             │              │
│             │               │ Data Key     │             │              │
└─────────────┘               └──────────────┘             └──────────────┘
```

---

## API Reference

All endpoints require `Authorization: Bearer <token>` header.

### Secrets Management
```bash
# Store a secret
PUT /v1/secrets/:key
Content-Type: application/json
{ "secret": "value", "ttl": 3600 }

# Retrieve a secret (latest version)
GET /v1/secrets/:key

# Retrieve specific version
GET /v1/secrets/:key?version=1

# Delete a secret version
DELETE /v1/secrets/:key?version=1
```

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

- [ ] JWT authentication with replay protection
- [ ] Automatic TTL expiration cleanup  
- [ ] Web UI for secret management
- [ ] Docker and Kubernetes deployment guides
- [ ] Multi-node replication support

## Support

- 📖 [Documentation](https://github.com/realmorrisliu/sealbox/wiki)
- 🐛 [Issue Tracker](https://github.com/realmorrisliu/sealbox/issues)
- 💬 [Discussions](https://github.com/realmorrisliu/sealbox/discussions)

---

## License

Apache License 2.0
