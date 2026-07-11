# Multi-Tenant Key Isolation Plan

## Status

- Implemented on 2026-07-10.
- Tenant repositories, hashed API tokens, transactional legacy migration,
  consistent pre-migration backup/reporting, tenant-scoped v2 APIs, CLI
  administration, and two-tenant live validation are complete.
- v1 remains the explicit legacy namespace compatibility surface. New
  integrations use v2 tenant tokens and per-tenant key pairs; operators can
  disable v1 with `LEGACY_V1_ENABLED=false`.

## Problem

Sealbox currently has one static bearer token, one globally active RSA master
key, and unscoped secret APIs. The `secrets.namespace` column exists, but most
repository operations ignore it and clients always write an empty namespace.
Consequently, registering multiple public keys does not create independent
user stores: the server permits only one active key globally and list/get/delete
operations are keyed without an authenticated tenant scope.

Universal Agents needs multiple private users in one instance. Each user's
credentials must be encrypted to a distinct key pair, and a credential request
authenticated for one user must not list, fetch, overwrite, rotate, or delete
another user's records or plaintext metadata.

## Security Decisions

- Treat a Sealbox tenant as one cryptographic and authorization boundary.
- Resolve tenant identity from an opaque bearer token on the server. Never
  trust a tenant id supplied in a request body, query, or path for data access.
- Keep one active RSA master key per tenant, rather than one globally active
  master key.
- Scope every secret and master-key repository operation by authenticated
  tenant id, including list, history, cleanup, rotation, import, and export.
- Keep encryption and decryption client-side. The server receives public keys,
  encrypted data, encrypted data keys, and plaintext metadata, but no private
  keys or secret plaintext.
- Store only hashes of high-entropy tenant bearer tokens. Return or write a new
  token only once during provisioning.
- Preserve a separate root/admin token for tenant lifecycle operations. Root
  credentials must not be accepted accidentally as an ordinary tenant data
  token.
- Continue treating plaintext credential metadata, including usernames, as
  private tenant data even though it is not encrypted.
- Use tenant suspension and token revocation for access removal. Do not delete
  encrypted records or key history implicitly when a tenant is disabled.

Per-tenant key pairs are defense in depth, not a replacement for authorization.
A shared broker that holds every tenant private key remains a trusted component.
If a deployment requires private keys to be held only by end users, unattended
retrieval and scheduled automation cannot run without a separate unlock, key
agent, KMS, or HSM workflow.

## Compatibility Strategy

- Add tenant-scoped behavior under the v2 API while retaining the current v1
  single-tenant API during migration.
- Map existing v1 data, the existing static token, and the existing active key
  to a reserved `legacy` tenant.
- Keep v1 explicitly disableable; Universal Agents managed deployments disable
  it while Sealbox's standalone default remains compatible with existing v1
  clients.
- Do not infer a tenant from a secret-key prefix. Prefixes may aid operations,
  but authenticated scope remains authoritative.

## Persisted Model

Add versioned, transactional SQLite migrations before changing the live schema.

### Tenants

Create a `tenants` table with:

- `id`: opaque stable id, primary key
- `status`: active or suspended
- `display_name`: optional operator-facing label
- `created_at`, `updated_at`
- non-secret metadata for integration ownership, if needed

Do not use mutable usernames, email addresses, or channel ids as tenant primary
keys.

### API Tokens

Create an `api_tokens` table with:

- `id`: token lookup id
- `tenant_id`: nullable only for root/admin credentials
- `token_hash`: hash of a high-entropy random token
- `role`: tenant_data or root_admin
- `created_at`, `expires_at`, `revoked_at`, `last_used_at`
- optional non-secret label

Use a token format with a non-secret lookup id and random secret component so
authentication does not scan every token hash. Compare the stored and supplied
hashes in constant time.

### Master Keys

Add `tenant_id` to `master_keys` and replace the global active-key unique index
with a partial unique index over `(tenant_id, status)` for `status = 'Active'`.
All create/list/get/active/rotation operations must require the authenticated
tenant scope.

### Secrets

Use the existing `secrets.namespace` value as the persisted tenant scope for
the first migration, or rebuild the table with an explicit `tenant_id` column
if the schema migration layer makes that safer. In either case:

- all primary-key, version, prune, expiry, list, history, get, save, delete, and
  rotation queries include tenant scope;
- a saved secret's `master_key_id` must reference an active key belonging to
  the same tenant;
- list and history responses never include another tenant's plaintext metadata;
- cleanup may run globally only as a root maintenance operation, while normal
  tenant cleanup remains scoped.

The current repository helpers that filter only by `key` must be treated as
unsafe and removed or made impossible to call without tenant scope.

## Authentication Context

Replace `static_auth` for v2 business routes with middleware that validates the
bearer token and inserts an immutable `AuthPrincipal` request extension:

```text
AuthPrincipal {
  token_id,
  role,
  tenant_id,
}
```

Handlers pass `tenant_id` explicitly into repository methods. Repository APIs
must not accept an optional tenant for ordinary data access; missing scope is an
error. Root-only tenant and token administration routes check `role` separately.

## API And CLI Surface

Add v2 tenant-scoped equivalents for secret and master-key operations. Data
routes do not need a tenant path parameter because the token supplies scope.

Add root-admin lifecycle operations for:

- tenant create/list/get/suspend/resume
- tenant token create/revoke/list-metadata
- optional tenant purge as a separate destructive, confirmed operation

Extend `sealbox-cli` with:

- tenant administration commands that use the configured root token;
- safe one-time token-file creation with mode `0600`;
- tenant-token configuration through file paths, not command-line values;
- tenant-aware key register/status/rotate operations;
- status output that reports the authenticated tenant id without exposing a
  token or public-key material unnecessarily.

The existing per-invocation key-path flags and environment variables remain
useful: a broker can select the tenant token file and matching public/private
key files for each call without placing secret material in argv.

## Migration

1. Introduce a schema-migration table and transactional migration runner.
2. Create the reserved `legacy` tenant.
3. Assign existing master keys and secret rows to `legacy`.
4. Register the existing static token as a legacy tenant token, or require an
   explicit one-time conversion command when automatic conversion would be
   unsafe.
5. Replace the global active-key index with the tenant-scoped index.
6. Verify every secret references a master key in the same tenant before
   committing the migration.
7. Provide a dry-run/report command with row counts, orphan checks, and no
   secret plaintext.
8. Back up the SQLite file before migration and document rollback as restoring
   that backup; do not attempt a lossy reverse migration after multiple tenants
   exist.

## Implementation Phases

### Phase 1: Schema Migrations And Tenant Repositories

- Add migration infrastructure.
- Add tenant and API-token repositories.
- Tenant-scope master-key and secret repository contracts and SQL.
- Add cross-tenant invariant tests at the repository layer.

### Phase 2: V2 Authentication And API

- Add `AuthPrincipal` middleware.
- Add root administration routes.
- Add tenant-scoped v2 secret and key routes.
- Ensure all error responses avoid confirming another tenant's record exists.
- Add structured audit events containing tenant id, token id, action, record
  key hash or safe identifier, outcome, and request id, but never secret values.

### Phase 3: CLI Tenant Support

- Add root and tenant token-file configuration.
- Add tenant lifecycle/token commands.
- Move secret, credential, key, rotation, import, and export commands to v2
  when tenant mode is configured.
- Verify token and private-key files remain out of argv, stdout, and logs.

### Phase 4: Legacy Migration And Compatibility

- Implement legacy-store detection and migration reporting.
- Keep v1 behavior only for the reserved legacy tenant.
- Add explicit configuration to disable v1 after migration.
- Update Docker, CLI, API, backup/restore, and security-model documentation.

### Phase 5: Universal Agents Integration Gate

Before declaring the Sealbox work complete, validate with Universal Agents that
one shared server can provision two tenant identities, register two distinct
public keys, and perform broker calls with per-invocation token/key environments.
Universal Agents owns the mapping from its immutable `canonical_user_id` to an
opaque Sealbox tenant id; Sealbox must not depend on UA channel identities.

## Verification Plan

- Unit tests for token hashing, lookup, expiry, revocation, and constant-time
  comparison behavior.
- Repository tests proving identical secret keys can exist independently in
  two tenants and that every read/write/list/history/delete/prune/rotation query
  is tenant-scoped.
- API tests proving tenant A receives not-found or forbidden responses for
  tenant B records without existence disclosure.
- Tests proving a tenant cannot register or activate a key for another tenant,
  save with another tenant's `master_key_id`, or rotate another tenant's data.
- Tests proving tenant A's private key cannot decrypt tenant B's encrypted data
  key and vice versa.
- Migration tests from a populated legacy database, including multiple secret
  versions, credential metadata, expired records, and retired keys.
- CLI tests for per-tenant key status, registration, set/get/list/delete,
  credential operations, import/export, and rotation.
- Run `cargo fmt --all -- --check`, strict workspace clippy, and
  `cargo test --workspace`.
- Run a live two-tenant server test across restart and token revocation before
  enabling the Universal Agents multi-user credential migration.

## Rejected As The Primary Design

- **Key prefixes with one global key:** useful as an application convention but
  provides no cryptographic separation and cannot secure metadata listing.
- **Multiple keys with one shared static token:** prevents decryption with the
  wrong private key but does not authorize list/get/delete operations or select
  an active key safely.
- **One Sealbox server and database per user:** works without Sealbox code
  changes and is a valid high-isolation deployment option, but process, port,
  health-check, backup, and lifecycle overhead grows linearly with users.
- **User-held private keys only:** strongest end-user custody, but incompatible
  with unattended broker hydration unless an online key agent, unlock workflow,
  KMS, or HSM is added.

## Resolved Decisions

- The existing `secrets.namespace` column is the tenant scope and is mandatory
  in repository contracts.
- The server generates a high-entropy token, stores its SHA-256 hash, and
  returns plaintext once; the CLI writes it to a new private file.
- v1 remains available for the reserved `legacy` namespace while v2 root and
  tenant authentication surfaces remain disjoint.
- Tenant purge is not included. Suspension and token revocation are the
  non-destructive lifecycle controls.
