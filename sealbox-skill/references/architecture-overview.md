# Sealbox architecture overview

Sealbox is a Rust + SQLite secret service with three collaborating parts:

- `sealbox-server`: authorization, persistence, route handling, and secret/version metadata.
- `sealbox-cli`: key generation and local crypto workflows, including create/read/delete/import/export orchestration.
- `sealbox-web`: authenticated web UI with real API integration for operational workflows.

## Core design

- Sensitive secret values are encrypted on the client before leaving the machine.
- The server stores encrypted payloads and metadata, not plaintext secret values.
- The REST API is token-gated. Business routes require a Bearer token.
- v1 routes use a server/static token model. v2 routes use tenant tokens and namespace secrets by tenant ID.
- Public key registration and rotation are first-class, while private keys remain local assets for decrypt/unwrap tasks.

## Security boundaries

1. Client creates data key and encrypts plaintext values (AES-GCM).
2. Client encrypts the data key with an active RSA public key.
3. Client sends encrypted envelope and metadata to server.
4. Client reads and decrypts by retrieving envelopes and unwrapping via local private key.

Only operations that need plaintext value access (read/decrypt/re-encipher during rotate) require private key possession.

## Data model boundaries

- Secret metadata includes key, version, tenant association (v2), status, created/updated/deleted timestamps, and TTL/expiry fields.
- Versioned storage allows history retrieval and targeted version deletes.
- Cleanup handles expired records lazily on read and at startup; manual cleanup is also available.

## Responsibility split

- CLI owns cryptographic correctness and UX around secrets, credentials, and local files.
- Server owns route behavior, persistence invariants, and policy enforcement.
- Web UI owns visualization, operational actions, and auth session handling.

## Operational implications

- Do not infer decryptability from list calls; list returns metadata.
- Credential usernames are usually stored as metadata for filtering and lookup.
- Tenant operations are intentionally separate from root/token-only operations.
- Legacy and new APIs are both part of the service surface, so compatibility depends on API version selection.
