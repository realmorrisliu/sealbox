# Maintenance and troubleshooting guide

## Readiness and health

- `GET /healthz/live` should return quickly when process is alive.
- `GET /healthz/ready` validates DB readiness as well as process readiness.
- If readiness fails:
  - check `STORE_PATH` is writable/readable.
  - check `LISTEN_ADDR` binding permissions.
  - check server auth token expectations.

## Expiration cleanup

- Startup performs cleanup of expired entries.
- Expired values are also enforced on read.
- Manual cleanup endpoint:
  - `POST /v1/admin/cleanup-expired`
- TTL checks are deterministic from creation timestamp + TTL seconds.

## Migrations and compatibility

- Server exposes a command mode:
  - `sealbox-server migration-report --store-path <path>`
- Use this to inspect migration state before running bulk import/export or schema operations.
- `LEGACY_V1_ENABLED` helps control compatibility behavior during migration windows.

## Common failures and diagnosis

### Unauthorized (`401` family)

- Confirm `Authorization: Bearer <token>`.
- In v2 mode, confirm you are using a tenant token for tenant routes and not a root token.
- Confirm token has not expired or been revoked.

### Forbidden / tenant access failures

- Verify tenant is not suspended.
- Verify endpoint path version and route group (`v1` vs `v2`) matches token type.
- Verify tenant token was created under the same root namespace/instance.

### Key errors

- If reads fail after decryption errors:
  - confirm private key path.
  - confirm local private key matches server active key ID (`key status`).
- If writes fail:
  - verify public key exists and is registered.
  - verify key list shows an active key with no pending status mismatch.

### Command syntax / parse errors

- Use command help for current flags:
  - `sealbox-cli --help`
  - `sealbox-cli secret --help`
  - `sealbox-cli credential --help`
  - `sealbox-cli tenant --help`

### Import/export operational risks

- Use version-compatible archive format for target environment.
- Exported archives are encrypted but still sensitive; protect them like key-bearing artifacts.
- Do not import from untrusted sources because key IDs and metadata can disrupt local workflows.

## Safe operational runbook (minimal)

1. Check health.
2. Confirm token/URL/app-version.
3. Confirm key registration (`key list`, `key status`).
4. Perform requested operation with explicit version selection.
5. For failures, isolate with minimal repro command and endpoint log context.
