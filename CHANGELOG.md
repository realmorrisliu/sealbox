# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive test suite with 51 tests covering cryptographic operations, database layer, and API handlers
- Complete CI/CD pipeline with GitHub Actions
- Multi-platform builds (Linux, macOS, Windows)
- Docker support with optimized multi-stage builds
- Security scanning with cargo-audit, CodeQL, and Trivy
- Code coverage reporting with codecov
- Automatic dependency updates with Dependabot
- Performance benchmarks and code quality checks

### Changed
- Improved error handling throughout the codebase
- Enhanced logging and observability
- Refactored crypto module with better error types

### Security
- Added comprehensive cryptographic testing
- Implemented security scanning in CI pipeline
- Added vulnerability scanning for Docker images
- Secrets scanning with TruffleHog

## [0.1.0] - 2024-XX-XX

### Added
- Initial release of Sealbox
- End-to-end encryption with RSA + AES-GCM
- SQLite-based storage with secret versioning
- REST API for secret management
- Master key management and rotation
- CLI tool for key generation and registration
- Static token authentication
- TTL support for secrets
- Docker deployment support

### Features
- Envelope encryption architecture
- Multiple secret versions
- Master key rotation
- Simple REST API
- Single binary deployment
- Embedded SQLite storage
- CLI tools for management

[Unreleased]: https://github.com/your-username/sealbox/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/your-username/sealbox/releases/tag/v0.1.0