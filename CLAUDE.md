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

# Store a secret with TTL (expires in 3600 seconds / 1 hour)
./target/release/sealbox-cli secret set temp-secret "temporary-value" --ttl 3600

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
- **tower-http**: CORS support for web development

### CLI
- **clap**: CLI argument parsing and command structure
- **toml**: Configuration file parsing
- **rpassword**: Secure password input
- **tabled**: Table formatting for output
- **anyhow**: Error handling with context

### Web UI (sealbox-web)
- **React 19**: Modern React framework
- **TanStack Start**: Full-stack React framework
- **TanStack Query**: Data fetching and caching
- **TanStack Router**: File-based routing
- **Zustand**: Lightweight state management
- **React Hook Form + Zod**: Form handling and validation
- **TailwindCSS + shadcn/ui**: Modern design system
- **date-fns**: Date/time formatting and localization
- **react-i18next**: Internationalization framework
- **i18next-browser-languagedetector**: Automatic language detection

## API Endpoints

### Health Check Endpoints (No Authentication Required)
- `GET /healthz/live` - Liveness probe for Kubernetes
- `GET /healthz/ready` - Readiness probe with database connection check

### Business Endpoints (Require `Authorization: Bearer <token>` header)
- `GET /v1/secrets` - List all secrets with metadata (key, version, timestamps, TTL)
- `PUT /v1/secrets/:key` - Create secret version (supports TTL via `ttl` field)
- `GET /v1/secrets/:key[?version=N]` - Retrieve secret (automatic expiry check)
- `DELETE /v1/secrets/:key[?version=N]` - Delete secret version
- `POST /v1/master-key` - Register public key
- `GET /v1/master-key` - List public keys
- `PUT /v1/master-key` - Rotate keys
- `DELETE /v1/admin/cleanup-expired` - Manual cleanup of expired secrets

## Development Status

### Completed Features
- ✅ Complete CLI architecture with robust configuration management
- ✅ Full key management command set (generate, register, list, rotate, status)
- ✅ Secret management with server-side encryption and client-side decryption
- ✅ **TTL (Time-To-Live) support** - Lazy cleanup with automatic expiration
  - Secrets can be created with optional TTL (time in seconds)
  - Expired secrets are automatically deleted when accessed
  - Manual cleanup API for batch removal of expired secrets
  - Startup cleanup removes expired secrets on server restart
- ✅ Batch operations (import/export functionality framework)
- ✅ **Complete internationalization (i18n)** - Full multi-language support
  - **4 languages supported**: English (default), Chinese (中文), Japanese (日本語), German (Deutsch)
  - **react-i18next framework** with automatic language detection
  - **Smart language switching** with dropdown selector and country flags
  - **Full UI translation** including forms, buttons, error messages, and tooltips
  - **Date localization** with date-fns for proper regional date formatting
  - **Brand consistency** - "Sealbox" name preserved across all languages
  - **Persistent language preference** stored in localStorage
- ✅ Zero clippy warnings across entire codebase (strict linting)
- ✅ Comprehensive test coverage (77 test cases including TTL functionality)
- ✅ Parameter-driven config initialization with interactive fallback
- ✅ Automatic path expansion and standardized file locations
- ✅ Production-ready code quality and error handling
- ✅ **Optimized database layer** - serde_rusqlite integration for automatic serialization
  - Upgraded rusqlite from 0.36.0 to 0.37.0 for compatibility
  - Implemented serde_rusqlite for automatic Secret struct mapping
  - Follows official best practices: query_and_then() + from_row() for single records, from_rows() for batch queries
  - Eliminated manual field mapping code, improving maintainability and type safety
- ✅ **Web UI (sealbox-web)** - Modern React-based web interface
  - **⚠️ DEVELOPMENT STATUS: EARLY STAGE** - Web UI is incomplete and actively under development
  - **🎨 Modern Design System**
    - ✅ **Professional UI design** with clean, functional interface
    - ✅ **Component architecture cleanup** - Custom components organized in dedicated directories
    - ✅ **Structured component organization**: `i18n/`, `theme/`, `brand/`, `common/` directories
    - ✅ **shadcn/ui component library** with consistent design system
    - ⚠️ **UI functionality is largely mock/placeholder** - Most features display static data
  - **🌐 Internationalization Foundation**
    - ✅ **4-language support** (English, Chinese, Japanese, German)
    - ✅ **react-i18next framework** with comprehensive translation keys
    - ✅ **Language switching** with dropdown selector
    - ✅ **Date localization** with date-fns
  - **🔐 Basic Authentication**
    - ✅ **Login page** with server URL and token input
    - ✅ **Bearer token authentication** using Zustand store
    - ✅ **Auth guard** protecting routes
    - ⚠️ **No logout functionality** - Missing user session management
  - **📋 Secret Management Interface (Mock)**
    - ⚠️ **Displays placeholder data** - Not connected to real API
    - ⚠️ **Create dialog exists** but doesn't save secrets
    - ⚠️ **Version history** shows mock data
    - ⚠️ **No actual CRUD operations** implemented
  - **🔑 Master Key Management (Mock)**
    - ⚠️ **Lists placeholder keys** with status badges
    - ⚠️ **"Coming Soon" messages** for register/rotate operations
    - ⚠️ **No actual key operations** implemented
  - **🚧 Critical Missing Features**
    - ❌ **No real API integration** - All data is mock/placeholder
    - ❌ **No secret CRUD operations** - Create/read/update/delete
    - ❌ **No key management** - Register/rotate master keys
    - ❌ **No real TTL handling** - Time-based expiration
    - ❌ **No error handling** - API error responses
    - ❌ **No data persistence** - Changes don't save
    - ❌ **No server communication** beyond login
- ✅ **Kubernetes-standard health checks** - Production-ready monitoring
  - `/healthz/live` - Liveness probe for service availability
  - `/healthz/ready` - Readiness probe with database connection testing
  - No authentication required for health endpoints
  - Proper HTTP status codes and JSON responses

### Development Priorities

#### Immediate (Web UI Core Functionality)
1. **🔌 API Integration** - Connect Web UI to real sealbox-server endpoints
   - Replace mock data with real API calls to `/v1/secrets` and `/v1/master-key`
   - Implement proper HTTP client with error handling
   - Add loading states and error boundaries
   - Connect authentication flow to server verification

2. **📋 Secret Management CRUD** - Complete all secret operations
   - **Create**: Save new secrets via `PUT /v1/secrets/:key` with server-side encryption
   - **Read**: Fetch secret content via `GET /v1/secrets/:key` with client-side decryption
   - **Update**: Edit secrets (creates new version) with proper version handling
   - **Delete**: Remove secrets via `DELETE /v1/secrets/:key` with confirmation
   - **List**: Display real secret metadata from `GET /v1/secrets`

3. **⏰ TTL and Expiration Handling** - Implement time-based features
   - Real-time expiration status with countdown timers
   - TTL input and validation in create/edit forms
   - Expired secret warnings with visual indicators
   - Automatic cleanup integration

4. **📖 Version History** - Complete version management
   - Fetch real version data from server
   - Version comparison and rollback functionality
   - Version metadata (timestamps, changes)

5. **🔑 Master Key Management** - Implement key operations
   - Key registration form with public key upload
   - Key rotation workflow with validation
   - Key status management (active/retired/disabled)
   - Remove "Coming Soon" placeholders

6. **🔒 Enhanced Authentication** - Complete auth system
   - Proper logout functionality with session cleanup
   - Token expiry handling and refresh
   - Connection status monitoring
   - User session management

#### Secondary Features
7. **📤 Import/Export** - Bulk operations for secrets
8. **🧪 Integration Testing** - End-to-end testing suite
9. **📊 Monitoring Dashboard** - System health and metrics
10. **🔐 JWT Authentication** - Replace static token auth

#### Long Term
11. **🚀 Multi-node Support** - Raft consensus for high availability
12. **🔍 Advanced Search** - Full-text search and filtering
13. **👥 Multi-user Support** - User roles and permissions

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