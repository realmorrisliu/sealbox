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
- ✅ **Web UI (sealbox-web)** - Complete modern React-based web interface with full functionality
  - **🎯 Complete Secret Management** - Full CRUD operations implemented
    - ✅ **Create secrets** with TTL support and form validation
    - ✅ **View secret content** with secure display and copy functionality
    - ✅ **Edit secrets** with versioning (creates new versions)
    - ✅ **Delete secrets** with confirmation dialogs
    - ✅ **Real-time TTL status** with expiration warnings and countdown
  - **🔑 Master Key Management Interface** - Dedicated management page
    - ✅ **Master key listing** with status indicators (Active/Retired/Disabled)
    - ✅ **Navigation system** between Secret Management and Master Keys
    - ✅ **Responsive design** with mobile and desktop optimized layouts
  - **🌐 Complete Authentication & Integration**
    - ✅ **Bearer Token authentication** with server status monitoring
    - ✅ **Full API integration** with all existing server endpoints
    - ✅ **CORS support** for development environment
  - **🎨 Production-Ready UI/UX Design**
    - ✅ **4-language internationalization** (English, Chinese, Japanese, German)
    - ✅ **Modern shadcn/ui components** with consistent design system
    - ✅ **Mobile-responsive layouts** with Linear/Superhuman style principles
  - **2025 Modern Industrial UI Design** following Linear/Superhuman style principles:
    - Strict 8pt grid spacing system (64px→32px→16px→8px hierarchy)
    - Function-first color system with minimal gradient usage
    - 3-layer page architecture (Header→Content→Footer)
    - Fixed table row heights (h-12) with consistent spacing
    - Visual restraint with 150ms transition duration standard
    - Clean typography using Inter font with optimized tracking
  - **🎯 2025 UI/UX Excellence Achieved** - Complete overhaul from basic to professional
    - **Replaced primitive alerts/confirms** with shadcn/ui Dialog components
    - **Modern notification system** using Sonner toast notifications
    - **Professional loading states** with Skeleton components instead of text
    - **Global error boundary** with graceful failure handling and recovery
    - **Cleaned design system** removing duplicate CSS and visual complexity
    - **Full internationalization** for all new UI components (4 languages)
    - **Accessibility improvements** with proper ARIA labels and keyboard navigation
    - **Production-ready UX** meeting 2025 web application standards
- ✅ **Kubernetes-standard health checks** - Production-ready monitoring
  - `/healthz/live` - Liveness probe for service availability
  - `/healthz/ready` - Readiness probe with database connection testing
  - No authentication required for health endpoints
  - Proper HTTP status codes and JSON responses

### Development Priorities
1. **🔑 Master Key Operations** - Implement key registration and rotation in Web UI
2. **🔐 JWT Authentication** - Replace static token auth with JWT tokens
3. **🧪 Integration Testing** - Add end-to-end API testing suite
4. **📊 Monitoring & Logging** - Add structured logging and metrics collection
5. **🚀 Multi-node Support** - Raft consensus for high availability deployment
6. **📱 Mobile App** - Native mobile applications for iOS and Android

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