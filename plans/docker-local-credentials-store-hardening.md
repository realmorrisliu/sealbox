# Docker Local Credentials Store Hardening Plan

## Decisions

- Target Sealbox as a lightweight, single-node local credentials store for Docker.
- Keep SQLite and the Rust server/CLI architecture.
- Make CLI/private-key workflows primary.
- Keep the Web UI secondary until browser-side encryption and token storage are hardened.
- Support Docker-secret-style `_FILE` inputs for bootstrap secrets and key material.
- Build with the latest stable Rust toolchain while supporting Rust 1.95 as the minimum version.
- Use the latest available crypto crate versions unless an upstream prerelease blocks the build.

## Phase 1: Build And Container Baseline

- Status: complete in this pass.
- Set and enforce Rust 1.95 as the minimum supported toolchain.
- Updated crypto dependencies and adapted API usage.
- Added `_FILE` config loading for server secrets.
- Added Docker-secret-friendly CLI key path handling.
- Fixed Docker healthcheck to use `/healthz/ready`.
- Verified `cargo test --workspace`.

## Phase 2: Safe Server And Storage Semantics

- Status: complete in this pass.
- Replaced plaintext write handling with encrypted write payloads.
- Added deterministic active master-key selection.
- Removed server API support for private-key rotation payloads.
- Added client-side master-key rewrap flow.
- Fixed latest-secret TTL fallback.
- Fixed list metadata so all fields come from the selected latest row.

## Phase 3: CLI And API Parity

- Status: partial.
- Completed: implemented CLI secret listing from `GET /v1/secrets`.
- Completed: aligned master-key API response shape with clients.
- Remaining: implement secret history or add a matching server endpoint.
- Remaining: add replace/prune workflows for old versions.

## Phase 4: Web UI Scope And Docs

- Status: partial.
- Completed: stopped persisting bearer tokens in localStorage by default.
- Completed: disabled Web UI secret creation API calls until browser-side crypto is implemented.
- Completed: updated README and docs to state the actual local-store security model.
- Completed: documented Docker secrets and `_FILE` env vars.
- Remaining: document backup/restore and runtime volume expectations in more operational detail.

## Phase 5: Operational Hardening

- Status: not started.
- Add schema migrations.
- Add backup/restore and integrity-check workflows.
- Add request size limits.
- Replace broad CORS toggle with explicit allowed origins.
- Add minimal non-secret audit events.

## Verification Gates

- Passed: `rtk cargo fmt --all`
- Passed: `rtk cargo clippy --all-targets --all-features --workspace -- -D warnings`
- Passed: `rtk cargo test --workspace`
- Blocked locally: `pnpm`/`vite` are not installed, so the Web UI build could not run.
- Blocked locally: `rtk docker build -t sealbox:local .` hung while resolving Docker base-image metadata and was canceled.
- Remaining: container smoke test for health, key registration, encrypted set/get, TTL, restart persistence, and key rotation.
