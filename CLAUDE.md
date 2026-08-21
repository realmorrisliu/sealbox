# CLAUDE.md

Guidance for Claude Code working in this repository.

## Status

**The design is settled; the implementation is not.** This file, `README.md`, and everything in
`docs/` describe the **target** system. The code in this repository is the previous generation and
is being replaced in significant part.

| | |
|---|---|
| Target design | [`docs/agent-native-design.md`](docs/agent-native-design.md) |
| Decisions and their reasoning | [`docs/adr/`](docs/adr/) — 12 ADRs |
| Vocabulary — use these words, no synonyms | [`CONTEXT.md`](CONTEXT.md) |
| Behavior already specified | [`openspec/specs/`](openspec/specs/) — `http-api`, `master-key`, `secret-encryption` |

Work goes through OpenSpec: `/opsx:propose` to plan a change, `/opsx:apply` to build it,
`/opsx:archive` when its tasks are done. Archived changes keep their reasoning in
[`openspec/changes/archive/`](openspec/changes/archive/).

**`retire-legacy-surface` is done** — the four cleanups below are complete, along with the
server-held master key they turned out to require. Nothing else in the MVP has been built. When
you touch this repo you are building an MVP item, listed under **Work order** below.

## What sealbox is

**An isolation layer between agents and real credentials. An agent can *use* a credential without
ever *seeing* it.**

Agents bring a threat ordinary secret managers never modelled: **prompt injection**. An agent
reads a hostile issue and is talked into exfiltrating the token it just fetched. Encryption at
rest does not help. Only two things do, and they are the whole product:

- **No interface returns a secret's value.** There is no `get_secret`.
- **An agent cannot compose a command.** It invokes **grants** a human approved.

A **grant** is a permitted use of secrets: which secrets, what is done with them, on which runner.
Read the relationship backwards and it explains itself — *everything a secret can be used for is
exactly the set of grants that declare it*, and that set is a few readable lines. In Vault or
GitHub Secrets that set is unbounded. Collapsing unbounded into readable is the product.

## Architecture

Three components, strict split of responsibility (ADR 0008):

```
sealbox-server (hosted, Fly.io)   store · authorise · generate values · dispatch · audit
                                  NEVER executes anything, cannot reach your VPC
        ↑ long-poll (outbound)          ↑ HTTPS
sealbox runner (in your cluster)  claim a job, receive plaintext, execute, report
                                  the ONLY place plaintext exists outside the server
sealbox CLI (laptop / agent)      submit jobs, show results — NEVER receives plaintext
        ↓
your infrastructure               cluster API · RDS · OSS · GitHub
```

The server cannot execute: a hosted instance has no route into your VPC, and giving it one would
expose production to the internet. The runner polls **outbound**, so the cluster needs no inbound
port. The CLI is a remote control.

**Humans will authenticate with passkeys** (ADR 0009) — no admin credential on disk. *Not built
yet:* every identity currently holds a bearer token. Authorisation does not care how an identity
authenticated, so passkeys replace how a credential is presented, not what it resolves to.

**Authorisation lives in the router**, not in handlers: routes are grouped by required role and
each group carries its own gate, so a route not placed in a group is not routed at all. Adding an
endpoint without choosing a group makes it 404, never open.

## Vocabulary that changed

Getting these wrong makes the codebase incoherent:

| Use | Never | Why |
|---|---|---|
| **Grant** | task, job | A grant is standing permission; a **Job** is one execution of it. Both exist. |
| **Rotate** | — | Replacing a secret's **value**. |
| **Rekey** | rotate | Re-encrypting a data key under a different master key. The value does not change. |
| **Master key** | KEK, root key | One name at every layer. The table is already `master_keys`. |
| **Runner** | worker, executor, agent | "Agent" means the LLM client here. |

## Work order

### Done: `retire-legacy-surface`

The private-key rekey endpoint, `sealbox-web`, the CORS layer, the `Version` enum, and
`Secret.namespace` are all gone; `rotate_master_key` is now `rekey`; the repo traits own their
connection and mention no rusqlite type; routes hardcode `/v1/`.

Removing the client-supplied private key required the server to hold a master key of its own, so
**`SEALBOX_MASTER_KEY_PATH`** now exists — a comma-separated list, most-current first, so
rotating it does not deadlock on the private half needed to read the old secrets.
`master_keys.server_held` records which keys the server holds; a secret under any other key is
**cold** and cannot be decrypted here, by design. All server env vars now carry a `SEALBOX_`
prefix.

Two bugs surfaced while doing it, both fixed: rekey committed partial results despite the spec
requiring all-or-nothing, and `route_layer` had put the health probes behind authentication, so
Kubernetes probes had been failing wherever this was deployed.

An existing database from before this is not migrated — delete it and start again (ADR 0012).

### Then the MVP — ten items

Acceptance test: the author's own infrastructure runs on it.

1. **Server on Fly.io** — master key and SQLite on the volume, Litestream to object storage.
   The master key itself already exists; what remains is the hosting and the ceremony. Includes the initialisation ceremony: deploy-time bootstrap token (never logged, single use,
   time-boxed, zero-identity only) and recovery-keypair backup with **mandatory re-entry
   verification** (ADR 0010).
2. **`identities`** — *partly done.* Identities, three ordered roles, hashed tokens, revocation,
   the audit trail, and a bounded bootstrap all exist. **Still missing:** passkeys (ADR 0009),
   single-use invites for humans, join tokens for runners, and the `runner` role itself.
3. **`sealbox set` and `sealbox gen`.**
4. **Grants stored server-side**, including script bodies, with parameters, a declared runner, and
   three injection forms (env var, `0600` temp file, env-file). Execution is argv, never a shell.
   Two built-in adapters: `kubernetes-secret` and `postgres-role` (ADR 0007).
5. **`jobs` queue and `sealbox runner`** — claim-and-report, timeout marks abandoned jobs failed.
   **No automatic retries**: grants are not necessarily idempotent.
6. **`sealbox run <grant> [args]`.**
7. **`sealbox rotate <secret> --via <grant> [--from-output]`** — commits only on success.
   Rotation-capable adapters must **create-new-then-drop-old**, never mutate in place (ADR 0011).
8. **`audit` table and `sealbox audit`.**
9. **Passkey authentication for every admin operation** — server-rendered approval page, in-memory
   `sealbox admin` session so bulk import is one fingerprint (ADR 0009).
10. **A skill file plus `examples/grants/`** — worked examples are the template library.

### Deliberately not being built

Leases, an egress proxy, an import command (`for f in ...; do sealbox set ...; done` suffices), a
web UI, retries or scheduling in the job queue, and any provider- or Kubernetes-specific code
inside sealbox. `kubernetes-secret` is an ordinary adapter — that it needs no special support is
the proof the design is right.

## Repository layout

- **`sealbox-server/`** — `api/` (Axum handlers, routing, auth), `crypto/` (RSA + AES-GCM envelope
  encryption), `repo/` (SQLite via rusqlite + serde_rusqlite), `config.rs`, `error.rs`
- **`sealbox-cli/`** — `commands/`, `config.rs` (TOML + env overrides), `output.rs`. Reuses the
  server's crypto modules.
- **`sealbox-server/tests/`** — HTTP-level tests: routing, middleware, payload rejection.

## Commands

```bash
cargo build --release                # or -p sealbox-server / -p sealbox-cli
cargo test --workspace
cargo fmt
cargo clippy --all-targets --all-features --workspace -- -D warnings   # zero warnings required
```

## Conventions

- **Nothing is kept for backward compatibility** (ADR 0012). No migrations, no deprecation
  periods, no compatibility shims — an incompatible database is deleted and rebuilt. Use this
  freedom deliberately: a rename or a restructure costs nothing now and a great deal later, so
  make the breaking decision *now* rather than deferring it.
- **SQLite stays.** Writes are rare, reads cacheable, dataset is megabytes, the only growing table
  is append-only audit. Litestream covers durability. Postgres would cost the single-binary
  property and buy nothing.
- **Adapters are compiled in, not plugins.** Growth rule: an adapter is built in only once it
  would replace **two scripts that actually exist**. Never for an imagined need.
- **Provider logic never enters sealbox.** Percent-encoding, `CREATE ROLE`, cloud APIs — that is
  the adapter's or the script's job.
- Follow existing style; match surrounding comment density and naming.
