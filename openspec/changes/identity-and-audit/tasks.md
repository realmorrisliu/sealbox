## 1. The identity model

- [x] 1.1 Add a `Role` enum (`Agent` < `Operator` < `Admin`) with an ordering, and `ToSql`/`FromSql` matching how `MasterKeyStatus` is stored
- [x] 1.2 Add an `Identity` struct: id, name, role, token hash, created_at, revoked_at
- [x] 1.3 Create the `identities` table, with a unique index on the name and on the token hash
- [x] 1.4 Generate tokens as 256 bits from a CSPRNG, rendered with a `sealbox_` prefix; return the plaintext only from creation
- [x] 1.5 Store the SHA-256 of the token, never the token; compare in constant time
- [x] 1.6 `IdentityRepo`: create, find by token hash, list, revoke. No method returns a token.

## 2. The audit trail

- [x] 2.1 Add an `AuditRecord` struct: id, timestamp, identity name (denormalised so it survives revocation), action, resource, outcome, detail
- [x] 2.2 Create the `audit` table, indexed by timestamp and by identity name
- [x] 2.3 `AuditRepo`: append, and query filtered by identity, action, and time range — most recent first. **No update or delete method exists.**
- [x] 2.4 Confirm no field can carry a secret value: the record holds a resource *name*, never a value, and error detail is a message rather than a payload

## 3. Authentication

- [x] 3.1 Replace the token comparison in `api/auth.rs` with a lookup: hash the presented token, find a non-revoked identity, attach it to the request
- [x] 3.2 Return unauthorised for an unknown or revoked credential, disclosing nothing about the resource
- [x] 3.3 Remove `SEALBOX_AUTH_TOKEN` from `SealboxConfig` and the environment
- [x] 3.4 Add `SEALBOX_BOOTSTRAP_TOKEN`, read at startup and never logged

## 4. Authorisation

- [x] 4.1 Add a `require_role` layer that admits an identity whose role is at or above the required one, and returns **forbidden** — distinct from unauthorised — otherwise
- [x] 4.2 Group the routes by required role and apply the layer per group; keep the public routes registered last, after every auth layer
- [x] 4.3 Assign each existing endpoint its role: secrets read/list → agent; secret write/delete → operator; master-key create/rekey and admin cleanup → admin
- [x] 4.4 Confirm the default: a route added outside every group is not routed at all

## 5. Bootstrap

- [x] 5.1 Add an endpoint that creates the first admin, accepting the bootstrap token
- [x] 5.2 Enforce all three conditions: no identity exists, the token matches, and the window is open. The window is now `SEALBOX_BOOTSTRAP_WINDOW_SECS` (default 1800) rather than a hardcoded constant — it was untestable, and a shorter window is something an operator might legitimately want
- [x] 5.3 Return the new admin's token once; audit the creation against an empty trail
- [x] 5.4 Confirm the bootstrap token appears in no log line, response body, or table

## 6. Wiring audit into the request path

- [x] 6.1 Add middleware that records every business request. Merged with authentication into one layer — they need the same two things at opposite ends: the identity is resolved on the way in, the outcome only known on the way out. Separate layers would mean resolving the token twice or leaving refusals unrecorded
- [x] 6.2 Ensure refusals are recorded — both unauthenticated and forbidden — since those never reach a handler
- [x] 6.3 Fail the request if the audit write fails, rather than acting without a record
- [x] 6.4 Confirm the health probes are not audited and remain unauthenticated

## 7. CLI

- [x] 7.1 `sealbox-cli identity create <name> --role <role>` — prints the token once, with a warning that it will not be shown again
- [x] 7.2 `sealbox-cli identity list` and `identity revoke <name>`
- [x] 7.3 `sealbox-cli audit [--identity X] [--action Y] [--since D] [--limit N]`. `--since` takes `90s`/`30m`/`24h`/`7d` or a timestamp — relative is what anyone types when something has just gone wrong. Query strings are built through `Url` so an action like `PUT /v1/secrets/db-password` encodes correctly
- [x] 7.4 Config carries this machine's identity token; remove any assumption of a shared token
- [x] 7.5 `sealbox-cli bootstrap --token <value>` for claiming a fresh server

## 8. Tests

- [x] 8.1 The role matrix, per endpoint: for each of the three roles, assert permitted and refused
- [x] 8.2 A revoked identity is refused at once, and other identities keep working
- [x] 8.3 No interface returns a stored token; the creation response is the only place one appears
- [x] 8.4 Bootstrap: succeeds once on an empty database; refused when an identity exists; refused after the window
- [x] 8.5 A refused request produces an audit record naming the identity and the refusal
- [x] 8.6 An unauthenticated request is recorded without being attributed to an identity
- [x] 8.7 Audit records contain no secret value, including on a failure path
- [x] 8.8 Health probes stay unauthenticated and unaudited

> **Noted while smoke-testing, out of scope here:** `sealbox-cli secret list` prints "Server does
> not currently support listing all secrets" although `GET /v1/secrets` exists and works. The CLI
> is due to be rewritten for the new command surface; folded in there rather than patched now.

## 9. Documentation

- [ ] 9.1 Update `docs/configuration.md`: `SEALBOX_AUTH_TOKEN` gone, `SEALBOX_BOOTSTRAP_TOKEN` in, identity token in the CLI config
- [ ] 9.2 Update `docs/cli-reference.md` for the identity and audit commands that now exist
- [ ] 9.3 Update `docs/getting-started.md` so the first steps are bootstrap and identity creation
- [ ] 9.4 Update `CLAUDE.md`: MVP item 2 partially done — identities and audit yes, passkeys and enrolment flows not yet
- [ ] 9.5 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
