# Security model and threat boundaries

Sealbox is intentionally designed so ciphertext is created and mostly managed client-side.

## Envelope encryption flow

For writes:

1. CLI generates a random data key.
2. Value is encrypted with AES-256-GCM.
3. Data key is encrypted with current RSA public key.
4. API receives encrypted blob, metadata, and encrypted data key.

For reads:

1. Client fetches encrypted payload/version metadata.
2. Client uses local private key to unwrap AES data key.
3. Client decrypts value.

## Key ownership

- Public keys are uploaded to server and used for encryption/reencryption.
- Private keys should remain local-only and never uploaded.
- `key generate` produces key material in CLI-configured local paths.
- `key rotate` requires local/private-key access to perform unwrap and rewrap as needed.
- `key status` validates local key identity against registered key IDs.

## Tenant isolation model

- v2 tenant token maps to a tenant namespace.
- Namespace derivation is deterministic from the token and not user-supplied plain strings.
- Root/static token is treated as server admin and is not interchangeable with tenant secrets routes.
- Tenant routes reduce blast radius by preventing cross-tenant reads/writes under same API host.

## Authorization semantics

- `AUTH_TOKEN` is for API access control only.
- It is not used for symmetric encryption and cannot decrypt payloads alone.
- Bearer token leakage grants API access but not plaintext decryption unless private key is also available.

## TTL and cleanup

- TTL values are stored as expiry metadata and enforced during fetch/startup/cleanup.
- Expired records can remain in DB metadata until cleanup events run.
- `DELETE /v1/admin/cleanup-expired` triggers batch expiration enforcement.

## Operational security trade-offs

- Secret metadata (like credential usernames) may be intentionally stored in plaintext metadata fields for usability/searching.
- Do not interpret this as full plaintext confidentiality of all record fields.
- Always rotate keys and tokens if compromise is suspected.

## Recommended deployment security posture

- Use HTTPS in front of the API (reverse proxy or TLS server config).
- Restrict key file and config permissions.
- Keep tokens and private keys in separate trust scopes.
- Prefer automation with least-privilege tenant tokens for service workloads.
