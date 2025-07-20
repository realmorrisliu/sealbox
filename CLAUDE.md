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
**CRITICAL**: This project currently has NO TESTS. This is a major security concern for a secret storage service.

```bash
# Run tests (when they exist)
cargo test

# Run tests in specific package
cargo test -p sealbox-server

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

1. **ADD TESTS IMMEDIATELY** - Critical security gap for a secret storage service
2. Implement proper CI/CD pipeline  
3. Add comprehensive logging and monitoring
4. Implement TTL cleanup mechanism