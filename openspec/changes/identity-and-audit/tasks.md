## 1. The identity model

- [ ] 1.1 Add a `Role` enum (`Agent` < `Operator` < `Admin`) with an ordering, and `ToSql`/`FromSql` matching how `MasterKeyStatus` is stored
- [ ] 1.2 Add an `Identity` struct: id, name, role, token hash, created_at, revoked_at
- [ ] 1.3 Create the `identities` table, with a unique index on the name and on the token hash
- [ ] 1.4 Generate tokens as 256 bits from a CSPRNG, rendered with a `sealbox_` prefix; return the plaintext only from creation
- [ ] 1.5 Store the SHA-256 of the token, never the token; compare in constant time
- [ ] 1.6 `IdentityRepo`: create, find by token hash, list, revoke. No method returns a token.

## 2. The audit trail

- [ ] 2.1 Add an `AuditRecord` struct: id, timestamp, identity name (denormalised so it survives revocation), action, resource, outcome, detail
- [ ] 2.2 Create the `audit` table, indexed by timestamp and by identity name
- [ ] 2.3 `AuditRepo`: append, and query filtered by identity, action, and time range — most recent first. **No update or delete method exists.**
- [ ] 2.4 Confirm no field can carry a secret value: the record holds a resource *name*, never a value, and error detail is a message rather than a payload

## 3. Authentication

- [ ] 3.1 Replace the token comparison in `api/auth.rs` with a lookup: hash the presented token, find a non-revoked identity, attach it to the request
- [ ] 3.2 Return unauthorised for an unknown or revoked credential, disclosing nothing about the resource
- [ ] 3.3 Remove `SEALBOX_AUTH_TOKEN` from `SealboxConfig` and the environment
- [ ] 3.4 Add `SEALBOX_BOOTSTRAP_TOKEN`, read at startup and never logged

## 4. Authorisation

- [ ] 4.1 Add a `require_role` layer that admits an identity whose role is at or above the required one, and returns **forbidden** — distinct from unauthorised — otherwise
- [ ] 4.2 Group the routes by required role and apply the layer per group; keep the public routes registered last, after every auth layer
- [ ] 4.3 Assign each existing endpoint its role: secrets read/list → agent; secret write/delete → operator; master-key create/rekey and admin cleanup → admin
- [ ] 4.4 Confirm the default: a route added outside every group is not routed at all

## 5. Bootstrap

- [ ] 5.1 Add an endpoint that creates the first admin, accepting the bootstrap token
- [ ] 5.2 Enforce all three conditions: no identity exists, the token matches, and the server started under 30 minutes ago
- [ ] 5.3 Return the new admin's token once; audit the creation against an empty trail
- [ ] 5.4 Confirm the bootstrap token appears in no log line, response body, or table

## 6. Wiring audit into the request path

- [ ] 6.1 Add middleware that records every business request: identity (or none), action, resource from the path, outcome
- [ ] 6.2 Ensure refusals are recorded — both unauthenticated and forbidden — since those never reach a handler
- [ ] 6.3 Fail the request if the audit write fails, rather than acting without a record
- [ ] 6.4 Confirm the health probes are not audited and remain unauthenticated

## 7. CLI

- [ ] 7.1 `sealbox-cli identity create <name> --role <role>` — prints the token once, with a warning that it will not be shown again
- [ ] 7.2 `sealbox-cli identity list` and `identity revoke <name>`
- [ ] 7.3 `sealbox-cli audit [--identity X] [--action Y] [--since D]`
- [ ] 7.4 Config carries this machine's identity token; remove any assumption of a shared token
- [ ] 7.5 `sealbox-cli bootstrap --token <value>` for claiming a fresh server

## 8. Tests

- [ ] 8.1 The role matrix, per endpoint: for each of the three roles, assert permitted and refused
- [ ] 8.2 A revoked identity is refused at once, and other identities keep working
- [ ] 8.3 No interface returns a stored token; the creation response is the only place one appears
- [ ] 8.4 Bootstrap: succeeds once on an empty database; refused when an identity exists; refused after the window
- [ ] 8.5 A refused request produces an audit record naming the identity and the refusal
- [ ] 8.6 An unauthenticated request is recorded without being attributed to an identity
- [ ] 8.7 Audit records contain no secret value, including on a failure path
- [ ] 8.8 Health probes stay unauthenticated and unaudited

## 9. Documentation

- [ ] 9.1 Update `docs/configuration.md`: `SEALBOX_AUTH_TOKEN` gone, `SEALBOX_BOOTSTRAP_TOKEN` in, identity token in the CLI config
- [ ] 9.2 Update `docs/cli-reference.md` for the identity and audit commands that now exist
- [ ] 9.3 Update `docs/getting-started.md` so the first steps are bootstrap and identity creation
- [ ] 9.4 Update `CLAUDE.md`: MVP item 2 partially done — identities and audit yes, passkeys and enrolment flows not yet
- [ ] 9.5 Final verification: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`
