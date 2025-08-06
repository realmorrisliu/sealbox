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
  - **✅ PRODUCTION READY** - Fully aligned with sealbox-server capabilities
  - **🎨 Complete Design System**
    - ✅ **Professional UI design** with clean, functional interface
    - ✅ **Component architecture** - Well-organized components in `auth/`, `brand/`, `common/`, `i18n/`, `layout/`, `secrets/`, `theme/`, `ui/` directories
    - ✅ **shadcn/ui component library** with consistent design system
    - ✅ **TypeScript type safety** - Clean types matching server API exactly
  - **🌐 Complete Internationalization**
    - ✅ **4-language support** (English, Chinese, Japanese, German)
    - ✅ **react-i18next framework** with comprehensive translation coverage
    - ✅ **Automatic language detection** and persistent preference storage
    - ✅ **Date localization** with date-fns integration
  - **🔐 Complete Authentication System**
    - ✅ **Login page** with server URL and token validation
    - ✅ **Real server connection** testing via `/healthz/ready` endpoint
    - ✅ **Zustand authentication store** with persistent session management
    - ✅ **AuthGuard route protection** with automatic redirects
    - ✅ **Logout functionality** with session cleanup
    - ✅ **Comprehensive error handling** for connection and auth failures
  - **📋 Secret Management Interface**
    - ✅ **Full CRUD operations** - Create, Read (list), Delete secrets
    - ✅ **Real API integration** - Uses actual sealbox-server endpoints
    - ✅ **Complete API client** - Full coverage of `/v1/secrets` endpoints
    - ✅ **Secret listing** - Displays key, version, status, timestamps
    - ✅ **Secret creation** - Create new secrets with optional TTL
    - ✅ **Secret deletion** - Delete specific secret versions
    - ✅ **Table and card views** - Responsive design with view mode toggle
    - ✅ **Search functionality** - Client-side filtering by key name
    - ✅ **TTL countdown timers** - Real-time expiration display
    - ✅ **Status indicators** - Active, expiring, expired states
  - **🔑 Master Key Management (API Ready)**
    - ✅ **Complete API client** - Full `/v1/master-key` endpoint coverage
    - ✅ **React Query hooks** - Ready for create/rotate/list operations
    - ⚠️ **UI implementation pending** - Forms and workflows need completion
- ✅ **Kubernetes-standard health checks** - Production-ready monitoring
  - `/healthz/live` - Liveness probe for service availability
  - `/healthz/ready` - Readiness probe with database connection testing
  - No authentication required for health endpoints
  - Proper HTTP status codes and JSON responses

### Recent Improvements (2025-08-06)

- ✅ **SSR Hydration Issues Resolved** - Comprehensive fix for server-side rendering
  - **SSR-safe translations**: Created `useSSRSafeTranslation` hook with automatic English fallback from locale files
  - **Eliminated hydration mismatches**: Fixed all i18n-related water rendering inconsistencies
  - **Removed inline scripts**: Migrated from HTML inline JS to pure React component architecture
  - **Direct localStorage integration**: Theme and language preferences read directly in React components
  - **Zero hardcoded fallbacks**: All fallback translations sourced from `en.json` locale file
  - **Clean component APIs**: Simplified translation calls back to `t("key")` format
  - **Production-ready SSR**: Web UI now fully compatible with server-side rendering

- ✅ **Web UI Code Cleanup** - Aligned with sealbox-server capabilities
  - **Removed fictional features**: Eliminated environment labels, categories, risk levels, favorites, archives, access counts
  - **Simplified UI**: Clean interface showing only real server data (key, version, status, timestamps, TTL)
  - **Fixed API types**: Corrected health check response formats to match server
  - **Updated all language files**: Cleaned up translations in English, Chinese, Japanese, and German
  - **TypeScript compliance**: Zero compilation errors with streamlined type definitions
  
- ✅ **Production-Ready Web Interface**
  - **Secret Management**: Full CRUD operations matching server API exactly
  - **Authentication**: Token-based auth with server connection validation
  - **Internationalization**: 4-language support with clean translations
  - **Modern Tech Stack**: React 19, TanStack Start, TailwindCSS, shadcn/ui
  - **Real-time features**: TTL countdown timers and status indicators

- ✅ **sealbox-web Architecture Refactoring** - Improved maintainability and code quality
  - **Component architecture**: Refactored monolithic `SecretManagement` component (336→89 lines, -73%)
  - **Separation of concerns**: Business logic extracted to custom hooks (`useSecretManagement`, `useSecretFiltering`)
  - **Utility modules**: Created `lib/secret-utils.ts` for reusable functions (status calculations, data transformations)
  - **Component decomposition**: Split into 4 focused components
    - `SecretStats` - Statistics cards display
    - `SecretControls` - Search and view controls  
    - `SecretTable` - Table view implementation
    - `SecretCards` - Card view implementation
  - **Container-Presenter pattern**: `SecretManagement` now acts as lightweight coordinator
  - **Type safety**: Zero TypeScript errors, proper type definitions across all modules
  - **Code quality**: Prettier formatting, consistent code style
  - **Maintainability benefits**: 
    - Single responsibility components
    - Improved testability through logic separation
    - Enhanced reusability across different views
    - Cleaner debugging and development experience

### Development Priorities

#### Immediate
1. **🔑 Master Key Management UI** - Complete key management interface
   - **Key listing page**: Display registered master keys
   - **Registration workflow**: UI for uploading public keys
   - **Key rotation interface**: Implement rotation with validation
   - **Status indicators**: Show active/retired/disabled states

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