# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sealbox is a lightweight, single-node secret storage service built in Rust. It provides envelope encryption with end-to-end encryption (E2EE) using RSA key pairs, stores data in SQLite, and exposes a REST API for secret management.

### Key Architecture Components

- **sealbox-server/**: Main server application with REST API
  - `api/`: HTTP handlers and routing (Axum framework)
  - `crypto/`: Encryption/decryption logic (RSA + AES-GCM envelope encryption)
  - `repo/`: Data persistence layer (SQLite with rusqlite)
  - `config.rs`: Environment-based configuration
  - `error.rs`: Centralized error handling

- **sealbox-cli/**: Command-line interface for key management and secret operations
  - `commands/`: Command handlers (config, key, secret management)
  - `config.rs`: TOML-based configuration management with environment overrides
  - `output.rs`: Multi-format output support (JSON, YAML, Table)
  - **Note**: CLI reuses server's crypto modules for consistency

### Security Model
- **Server-side encryption**: CLI sends plaintext to server, server performs envelope encryption
- **Envelope encryption**: Secrets encrypted with random data keys, data keys encrypted with user's public key
- **RSA + AES-GCM**: 2048-bit RSA for key encryption, AES-256-GCM for data encryption
- **End-to-end security**: Only clients with private keys can decrypt stored secrets

## Common Development Commands

### Building the Project
```bash
# Build everything (release)
cargo build --release

# Build only server
cargo build --release -p sealbox-server

# Build only CLI
cargo build --release -p sealbox-cli

# Development build
cargo build
```

### Running the Server
```bash
# Set required environment variables
export STORE_PATH=/var/lib/sealbox.db
export AUTH_TOKEN=secrettoken123
export LISTEN_ADDR=127.0.0.1:8080

# Run server
./target/release/sealbox-server
```

### Testing and Quality
The project includes comprehensive unit tests (71 total: 51 server + 20 CLI) covering encryption, decryption, storage, API functionality, and CLI operations.

```bash
# Run all tests
cargo test --workspace

# Run tests in specific package
cargo test -p sealbox-server
cargo test -p sealbox-cli

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Lint code (strict mode - zero warnings)
cargo clippy --all-targets --all-features --workspace -- -D warnings
```

### CLI Usage
The CLI provides comprehensive secret management by interfacing with the server's encryption system:

```bash
# Initialize configuration with parameters (recommended)
./target/release/sealbox-cli config init \
    --url http://localhost:8080 \
    --token your-secure-token \
    --public-key ~/.config/sealbox/public_key.pem \
    --private-key ~/.config/sealbox/private_key.pem \
    --output table

# Generate RSA key pair (automatically saved to ~/.config/sealbox/)
./target/release/sealbox-cli key generate

# Register public key with server
./target/release/sealbox-cli key register

# Store a secret (sent as plaintext, encrypted by server)
./target/release/sealbox-cli secret set mypassword "super-secret-value"

# Retrieve and decrypt a secret
./target/release/sealbox-cli secret get mypassword

# Import secrets from file
./target/release/sealbox-cli secret import --file secrets.json

# Multiple output formats supported
./target/release/sealbox-cli key list --output table
```

## Configuration

### Server Configuration
Environment variables:
- `STORE_PATH`: SQLite database file path
- `LISTEN_ADDR`: Server listen address (e.g., 127.0.0.1:8080)  
- `AUTH_TOKEN`: Static bearer token for API authentication

### CLI Configuration
The CLI uses TOML configuration files with environment variable overrides:
- Config file: `~/.config/sealbox/config.toml`
- Supports server URL, authentication tokens, key paths, and output preferences
- Command-line arguments override config file and environment variables
- Automatic `~/` path expansion for key file paths
- Can be initialized with parameters or interactively

## Key Dependencies

### Server
- **axum**: HTTP server framework
- **rusqlite**: SQLite database interface
- **rsa**: RSA cryptography implementation
- **aes-gcm**: AES-GCM symmetric encryption
- **tokio**: Async runtime

### CLI
- **clap**: CLI argument parsing and command structure
- **toml**: Configuration file parsing
- **rpassword**: Secure password input
- **tabled**: Table formatting for output
- **anyhow**: Error handling with context

## API Endpoints

All endpoints require `Authorization: Bearer <token>` header:

- `PUT /v1/secrets/:key` - Create secret version
- `GET /v1/secrets/:key[?version=N]` - Retrieve secret
- `DELETE /v1/secrets/:key[?version=N]` - Delete secret version
- `POST /v1/master-key` - Register public key
- `GET /v1/master-key` - List public keys
- `PUT /v1/master-key` - Rotate keys

## Development Status

### Completed Features
- ✅ Complete CLI architecture with robust configuration management
- ✅ Full key management command set (generate, register, list, rotate, status)
- ✅ Secret management with server-side encryption and client-side decryption
- ✅ Batch operations (import/export functionality framework)
- ✅ All Chinese text converted to English for global compatibility
- ✅ Zero clippy warnings across entire codebase (strict linting)
- ✅ Comprehensive test coverage (71 test cases)
- ✅ Parameter-driven config initialization with interactive fallback
- ✅ Automatic path expansion and standardized file locations
- ✅ Production-ready code quality and error handling

### Development Priorities
1. **JWT authentication** - Replace static token auth with JWT
2. **TTL cleanup mechanism** - Automatic expiration of secrets
3. **Integration testing** - Add end-to-end API testing
4. **Monitoring and logging** - Add structured logging and metrics
5. **Multi-node support** - Raft replication for high availability

## CI/CD Pipeline

The project uses a streamlined GitHub Actions workflow optimized for MVP development:

### CI Workflow (.github/workflows/ci.yml)
- **Code Quality**: Format checking (rustfmt), linting (clippy)
- **Testing**: Unit tests, documentation tests
- **Security**: Dependency audit (cargo audit)
- **Build**: Release build verification on Linux

### Release Workflow (.github/workflows/release.yml)
- **Platform**: Linux x86_64 binary releases
- **Automation**: Triggered by git tags (v*) or manual dispatch
- **Artifacts**: Compressed binary archive with README and LICENSE

The CI/CD setup prioritizes simplicity and speed for rapid development cycles while maintaining essential quality checks.