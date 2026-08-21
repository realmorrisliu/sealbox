## Why

Sealbox is being rebuilt as an agent-native credential broker (`docs/agent-native-design.md`).
Before any of that is built, four pieces of the previous generation have to go — not as
preparation, but because each is wrong on its own terms today.

The urgent one is a security hole: `PUT /v1/master-key` requires the client to POST its **old
private key in the clear**. Anyone able to read server memory, request bodies, or logs during a
rotation window can decrypt every historical version of every secret. This makes the "end-to-end
encryption" claim in the current README false, and it cannot be fixed by hardening — the endpoint's
contract is the flaw.

The other three remove code that is already dead, already contradicted by a recorded decision, or
already blocking the work that follows. Doing them first keeps every later change small: this one
only deletes and renames, so it is verifiable by "everything still compiles and passes", with no
new behavior to reason about.

## What Changes

- **BREAKING** — `PUT /v1/master-key` no longer accepts `old_private_key_pem`. Re-encrypting data
  keys under a new master key (now called **rekey**) becomes a server-side operation using a master
  key the server holds. Clients can no longer trigger a rekey by supplying key material.
- **BREAKING** — CORS support is removed. The API serves no browser client.
- **BREAKING** — `Version::V2` and `Version::V3` are removed from the API version enum. Every
  handler already answers `InvalidApiVersion` for both; the enum advertises support that does not
  exist.
- Rename `Secret::rotate_master_key` → `rekey` throughout. `rotate` is reserved for replacing a
  secret's **value** (`CONTEXT.md`); having one word mean both makes audit records ambiguous the
  moment rotation exists.
- Delete the `sealbox-web` crate — ~4500 lines of TypeScript, four locale files, one pnpm
  workspace. Per ADR 0004 the interface is the CLI, and there is no read-only dashboard either.
- Move `rusqlite::Connection` out of the `SecretRepo` / `MasterKeyRepo` / `HealthRepo` trait
  signatures into the implementors; drop `state.conn_pool.lock()` from the HTTP handlers.
- Resolve `Secret.namespace`, which has been `String::new()` since it was introduced yet is part of
  the SQLite primary key: either give it a meaning or remove it (decided in design).

## Capabilities

### New Capabilities

- `master-key`: the lifecycle of the keypairs secrets' data keys are encrypted under — registration,
  which key is current, the server-held vs cold distinction, and rekeying without any client-supplied
  key material.
- `http-api`: the transport-level contract — which API versions exist, how requests are
  authenticated, and the absence of cross-origin access.

### Modified Capabilities

None. `openspec/specs/` is empty; this is the first change to record any.

## Impact

**Constrained by** ADR 0001 (broker over E2EE — the server holding a master key is what makes a
server-side rekey possible), ADR 0004 (no web UI), and `CONTEXT.md` on the rotate/rekey distinction.
Contradicts no recorded decision.

**Code**
- `sealbox-server/src/api/handler/master_key.rs` — `RotateMasterKeyPayload`, the rotate handler
- `sealbox-server/src/api/mod.rs` — CORS layer, `Version` enum
- `sealbox-server/src/repo/mod.rs` — trait signatures, `Secret::rotate_master_key`, `namespace`
- `sealbox-server/src/repo/sqlite/*` — implementors take ownership of the connection; the `secrets`
  primary key if `namespace` is removed
- `sealbox-server/src/api/handler/*` — no longer lock the connection pool
- `sealbox-cli/src/commands/key_commands.rs` — the rotate command's payload
- `sealbox-web/` — deleted

**Interfaces** — `PUT /v1/master-key` changes shape. `SEALBOX_ALLOW_CORS` stops being read. Any
client relying on `/v2` or `/v3` being *enumerable* sees no change, since both already error.

**Data** — a migration is required if `namespace` leaves the primary key.

**Not affected** — envelope encryption itself, secret storage and versioning, TTL and lazy cleanup,
the health endpoints.
