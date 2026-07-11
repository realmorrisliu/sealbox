# Server API and endpoint reference

This document maps Sealbox HTTP behavior at a task level.

## Base behavior

- Base URL: value in CLI config (`SEALBOX_URL`) or explicit server config.
- Authorization: `Authorization: Bearer <token>` for non-health routes.
- Content type for JSON requests: `application/json`.
- API versions:
  - `v1`: legacy/static-token workflows.
  - `v2`: tenant-scoped workflows; tenant token derives namespace by hashing bearer token.

## Health endpoints (no auth)

### `GET /healthz/live`
- Checks process availability only.

### `GET /healthz/ready`
- Includes DB connectivity check.
- Primary startup/provisioning readiness target for UIs and automation.

## v1 business endpoints (server token protected)

### Secrets

- `GET /v1/secrets`
  - List latest secret metadata for all keys.
- `PUT /v1/secrets/:key`
  - Create a new secret version.
  - Body contains encrypted secret envelope and metadata.
  - Optional TTL handling by `ttl` in seconds.
- `GET /v1/secrets/:key`
  - Fetch a secret version (latest by default).
  - Supports `?version=<N>`.
  - Expired entries are checked and cleaned as needed.
- `GET /v1/secrets/:key/history`
  - List version metadata for a key.
- `DELETE /v1/secrets/:key`
  - Delete all versions unless `?version=<N>` is set.

### Credentials (username + password records)

- `GET /v1/credentials`
  - List credential metadata with server-side filters.
- `PUT /v1/credentials/:key`
  - Create/replace a credential version.
- `GET /v1/credentials/:key`
  - Fetch one credential metadata record and encrypted password blob.
- `GET /v1/credentials/:key/history`
  - List credential version metadata.
- `DELETE /v1/credentials/:key`
  - Delete latest or specific version with `?version=<N>`.

### Master keys

- `GET /v1/master-key`
  - List registered keys and status.
- `PUT /v1/master-key`
  - Register initial public key.
- `PUT /v1/master-key/rotate` (or equivalent rotate route in implementation)
  - Rotate active key; update all data keys to the new active key where possible.

### Admin

- `DELETE /v1/admin/cleanup-expired`
  - Immediate cleanup of expired records.

## v2 secret and key endpoints (tenant token protected)

- Same functional resources as v1 but scoped by tenant derived from token:
  - `GET /v2/secrets`
  - `PUT /v2/secrets/:key`
  - `GET /v2/secrets/:key`
  - `GET /v2/secrets/:key/history`
  - `DELETE /v2/secrets/:key`
  - `GET /v2/credentials`
  - `PUT /v2/credentials/:key`
  - `GET /v2/credentials/:key`
  - `GET /v2/credentials/:key/history`
  - `DELETE /v2/credentials/:key`
  - `GET /v2/master-key`
  - `PUT /v2/master-key`
- Static root token is intentionally not valid for v2 tenant data operations.

## v2 tenant admin endpoints (root token protected)

- `GET /v2/admin/tenants`
  - List tenants.
- `POST /v2/admin/tenants`
  - Create tenant.
- `GET /v2/admin/tenants/:tenant_id`
  - Get tenant details.
- `POST /v2/admin/tenants/:tenant_id/suspend`
  - Suspend tenant.
- `POST /v2/admin/tenants/:tenant_id/resume`
  - Resume tenant.
- `POST /v2/admin/tenants/:tenant_id/tokens`
  - Create a new tenant access token.
- `GET /v2/admin/tenants/:tenant_id/tokens`
  - List tenant tokens.
- `DELETE /v2/admin/tenants/:tenant_id/tokens/:token_id`
  - Revoke a token.

## API design notes

- All data mutating endpoints are versioned per key where applicable.
- `output_format` conventions are client-side (CLI or UI), not server-specific.
- Expiry is enforced at read and during startup/cleanup operations.
- v1 and v2 are intentionally aligned for most behaviors to simplify migration and tooling.
- Secret/certificate material remains encrypted in DB; API never needs private key material.

## Error semantics (high-level)

- Missing/invalid auth token: unauthorized response from middleware.
- Missing/invalid keys or token mismatch: authorization/lookup failures depending on endpoint.
- Expired entries: omitted from active queries and subject to cleanup behavior.
- Invalid payloads or path values: request validation errors with structured JSON error format.
