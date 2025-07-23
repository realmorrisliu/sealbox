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

### Security Model
- End-to-end encryption: Users generate RSA key pairs, server only stores encrypted data
- Envelope encryption: Secrets encrypted with random data keys, data keys encrypted with user's public key
- No plaintext storage: Server never has access to decrypt user secrets

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
The project includes comprehensive unit tests (44 test cases) covering encryption, decryption, storage, and API functionality.

```bash
# Run all tests
cargo test

# Run tests in specific package
cargo test -p sealbox-server
cargo test -p sealbox-cli

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Lint code
cargo clippy
```

### CLI Usage
```bash
# Create and register master key
./target/release/sealbox-cli master-key create \
    --url http://localhost:8080 \
    --token secrettoken123 \
    --public-key-path my_public_key.pem \
    --private-key-path my_private_key.pem
```

## Configuration

Server configuration via environment variables:
- `STORE_PATH`: SQLite database file path
- `LISTEN_ADDR`: Server listen address (e.g., 127.0.0.1:8080)  
- `AUTH_TOKEN`: Static bearer token for API authentication

## Key Dependencies

- **axum**: HTTP server framework
- **rusqlite**: SQLite database interface
- **rsa**: RSA cryptography implementation
- **aes-gcm**: AES-GCM symmetric encryption
- **clap**: CLI argument parsing
- **tokio**: Async runtime

## API Endpoints

All endpoints require `Authorization: Bearer <token>` header:

- `PUT /v1/secrets/:key` - Create secret version
- `GET /v1/secrets/:key[?version=N]` - Retrieve secret
- `DELETE /v1/secrets/:key[?version=N]` - Delete secret version
- `POST /v1/master-key` - Register public key
- `GET /v1/master-key` - List public keys
- `PUT /v1/master-key` - Rotate keys

## Development Priorities

1. **Expand test coverage** - Add integration tests and API end-to-end testing
2. Add comprehensive logging and monitoring
3. Implement TTL cleanup mechanism
4. Add OpenAPI documentation specification

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