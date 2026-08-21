## Context

See proposal.md — Why.

Constraints that shape the approach:

- Every master key in an existing database was registered by a client submitting a public key. The
  server holds **no** private halves today, which is exactly why the rotate endpoint demanded one
  from the caller.
- `Secret.namespace` is part of the `secrets` primary key and has been `String::new()` since it was
  introduced. SQLite cannot drop a column that participates in a primary key; removing it means
  rebuilding the table.
- The repo traits take `&rusqlite::Connection` / `&mut rusqlite::Connection` as a parameter, so every
  HTTP handler holds the connection mutex for the duration of its work.
- Authentication is a single static `AUTH_TOKEN`. Replacing it is a later change; this one must not
  depend on it.

## Goals / Non-Goals

**Goals:**

- Remove the endpoint that requires clients to transmit private key material, without leaving rekey
  a dead feature.
- Introduce the minimum needed for a server-side rekey: a server-held master key.
- Leave the workspace with no dead API versions, no unused primary-key column, and no
  browser-facing surface.
- Keep every step verifiable by "compiles, clippy clean, tests pass" — this change adds no
  externally visible behavior beyond what it removes.

**Non-Goals:**

- Identities, roles, passkeys — the static `AUTH_TOKEN` stays exactly as it is.
- Grants, jobs, runners, adapters.
- Making the repo traits async. The connection moves inside the implementors; the signatures stay
  synchronous.
- Migrating existing secrets onto a server-held master key. See Migration Plan.

## Decisions

### Introduce `server_held` in this change, not a later one

Deleting `old_private_key_pem` leaves rekey with no key material to work with. Either rekey becomes
temporarily impossible, or the server holds a private half. The spec written for this change
requires working rekey, so the second.

Scope is one column (`master_keys.server_held`), one config value (`SEALBOX_MASTER_KEY_PATH`), and
loading that key at startup. No new cryptography: `crypto::master_key::PrivateMasterKey` already
exists — it was what the rotate handler parsed out of the request body.

*Alternative rejected:* ship the deletion alone and restore rekey later. It would leave the
repository in a state where a documented capability does not work, and the follow-up change would
have to re-open the same files anyway.

### The two-tier model falls out of existing data

A secret is broker-serviceable if its master key is `server_held = 1`, and cold if it is not. No
`sealed` flag on secrets, no second code path, no new abstraction — the distinction is a property of
the key a secret already references (ADR 0001).

### Remove `namespace` rather than give it meaning

It has never held a value. Giving it one now would be inventing a requirement to justify a column;
the multi-tenancy it was presumably intended for is out of scope and, when it arrives, the recorded
answer is one SQLite file per tenant rather than a discriminator column.

Removal means rebuilding `secrets` with `PRIMARY KEY (key, version)`, inside a transaction, with the
old table dropped only after the copy succeeds.

*Alternative rejected:* leave it. A column that is always empty yet participates in the primary key
misleads every future reader about whether the table is tenant-scoped.

### Repo implementors own the connection

`SqliteSecretRepo { conn: Arc<Mutex<Connection>> }`, and the trait becomes
`fn get_secret(&self, key: &str) -> Result<Secret>`. Handlers stop calling `conn_pool.lock()`.

This is worth doing on its own merits — a database lock never belonged in the HTTP layer — and it
also stops the trait from leaking a type no non-embedded backend could satisfy.

Multi-statement work (rekey, version creation) takes the lock and opens a transaction inside the
implementor, where it belongs.

*Alternative rejected:* making the traits async at the same time. Nothing here needs it, and it
would touch every call site for no present benefit.

### Version lives in the route, not in a type

Removing V2 and V3 leaves `Version` with a single variant, at which point the dynamic `{version}`
path segment, the extractor, and every handler's match on it are noise: a one-armed match, and a
path parameter no handler reads. Clippy correctly flags the result as unused.

So the routes hardcode `/v1/...`. An unsupported version no longer produces a deserialization
failure inside an extractor — it simply matches no route and returns 404, identically for a version
that was once planned and one that never existed. The specs require rejection, not a particular
status code.

This also removes `MasterKeyPathParams`, `ListSecretsPathParams`, `SecretPathParams::version`, and
the `InvalidApiVersion` error variant, which after this has no producer.

*Alternative rejected:* keep the extractor and silence clippy with `_params`. That hides the
problem rather than removing it, and leaves a type whose only purpose is to be constructed and
discarded. When a v2 does arrive, a second route is clearer than a match inside a shared handler.

### CORS is deleted, not made configurable

Not a setting defaulting to off: the layer and `SEALBOX_ALLOW_CORS` both go. A configuration switch
implies a supported configuration, and per ADR 0004 there is no browser client to support. The
current `cfg!(debug_assertions)` branch means debug builds already behave differently from release —
precisely the kind of divergence that produces a surprise in production.

## Risks / Trade-offs

- **Existing secrets become unreachable by the server** → They remain readable by a CLI holding the
  corresponding private key, which is the cold path working as designed. See Migration Plan.
- **The `secrets` table rebuild could lose data** → Single transaction; copy and verify row counts
  before dropping; the file is the only copy, so back it up first. The table is small.
- **Rekey semantics change shape for the CLI** → `sealbox-cli`'s rotate command is updated in the
  same change; there is no other client.
- **A server-held master key means the server can decrypt** → The intended consequence of ADR 0001,
  not a regression. Credentials that must survive server compromise stay on cold keys.
- **`SEALBOX_MASTER_KEY_PATH` becomes a new operational requirement** → Startup fails loudly with a
  clear message if it is missing or unreadable, rather than degrading to a server that cannot rekey.

## Migration Plan

1. **Back up the database file.** It is the only copy.
2. **Schema migration**, in one transaction: add `master_keys.server_held` defaulting to `0`, then
   rebuild `secrets` without `namespace` and with `PRIMARY KEY (key, version)`.
3. **Generate and install a server master key**, register it with `server_held = 1`, and point
   `SEALBOX_MASTER_KEY_PATH` at it. It becomes the key new secrets are encrypted under.
4. **Existing secrets are not migrated.** Every pre-existing master key is cold — the server never
   had its private half — and rekey deliberately refuses a cold source, because accepting one would
   mean re-adding the endpoint this change exists to remove.

   Existing values are recovered by reading them with the CLI that holds the private key and calling
   `set` again. This is acceptable because the only deployment is the author's, the volume is small,
   and the MVP's acceptance scenario is a fresh import regardless.

**Rollback:** revert the binary and restore the backed-up database file. The schema migration is not
backward-compatible, so rollback is file-level, not schema-level. Nothing outside sealbox depends on
these tables.

## Open Questions

None that can be deferred. The one genuinely open item — how existing secrets reach a server-held
key — is answered above by declining to migrate them.
