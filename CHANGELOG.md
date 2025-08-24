# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Web UI (sealbox-web)** - Complete React-based web interface for secret management
  - Modern authentication system with Bearer Token support
  - Responsive secret list with real-time TTL status indicators
  - Secret deletion with confirmation dialogs
  - Mobile-friendly responsive design built with TailwindCSS and shadcn/ui
  - Integration with TanStack Query for efficient data fetching and caching
  - **English-first interface** - All UI elements use clear English text
- **Kubernetes Health Checks** - Production-ready monitoring endpoints
  - `GET /healthz/live` - Liveness probe for service availability
  - `GET /healthz/ready` - Readiness probe with database connection testing
  - No authentication required for health endpoints
  - Proper HTTP status codes and JSON responses
- **Complete English Internationalization** - Full language standardization
  - All UI components, error messages, and user-facing text in English
  - All code comments and documentation in English
  - English locale (enUS) for date formatting throughout the application
  - Prepared foundation for future multi-language i18n support
- **New API Endpoint**: `GET /v1/secrets` - List all secrets with metadata
- **CORS Support** - Cross-origin request handling for web development
- Comprehensive test suite with 77 tests covering cryptographic operations, database layer, and API handlers
- Complete CI/CD pipeline with GitHub Actions
- Multi-platform builds (Linux, macOS, Windows)
- Docker support with optimized multi-stage builds
- Security scanning with cargo-audit, CodeQL, and Trivy
- Code coverage reporting with codecov
- Automatic dependency updates with Dependabot
- Performance benchmarks and code quality checks

### Changed
- Enhanced API architecture to support web interface requirements
- Improved error handling throughout the codebase
- Enhanced logging and observability
- Refactored crypto module with better error types
- Upgraded tower-http dependency with CORS feature support

### Security
- Added comprehensive cryptographic testing
- Implemented security scanning in CI pipeline
- Added vulnerability scanning for Docker images
- Secrets scanning with TruffleHog
- Secure token-based authentication for web interface

## [0.1.0] - 2024-XX-XX

### Added
- Initial release of Sealbox
- End-to-end encryption with RSA + AES-GCM
- SQLite-based storage with secret versioning
- REST API for secret management
- Client key management and rotation
- CLI tool for key generation and registration
- Static token authentication
- TTL support for secrets
- Docker deployment support

### Features
- Envelope encryption architecture
- Multiple secret versions
- Client key rotation
- Simple REST API
- Single binary deployment
- Embedded SQLite storage
- CLI tools for management

[Unreleased]: https://github.com/realmorrisliu/sealbox/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/realmorrisliu/sealbox/releases/tag/v0.1.0
