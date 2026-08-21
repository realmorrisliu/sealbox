## Why

Authentication is a single static token compared against `SEALBOX_AUTH_TOKEN`. Every caller —
the operator, each agent, and eventually each runner — presents the same string. That has three
consequences, and each blocks something the MVP needs:

- **The audit log cannot exist.** Recording "who did this" is meaningless when everyone is the
  same principal. The whole point of letting an agent act is being able to answer, afterwards,
  which agent did what.
- **Nothing can be revoked in isolation.** Withdrawing one agent's access means rotating the
  token and reconfiguring every other caller.
- **Least privilege is unexpressible.** An agent that should only run approved grants holds
  exactly the same credential as the human who approves them.

Grants (`docs/agent-native-design.md`) depend on all three: approving a capability is an admin
act, invoking one is an agent act, and the distinction has to be real before either is built.

This change is deliberately *not* passkeys. Identities and their audit trail come first because
grants need them; how a human proves they are the admin is [ADR 0009](../../../docs/adr/0009-admin-authenticates-with-passkeys.md)
and comes next. Until then, admin identities hold a token like everyone else.

## What Changes

- **BREAKING** — `SEALBOX_AUTH_TOKEN` is removed. Every caller authenticates as a named identity
  holding its own token. There is no shared credential and no fallback.
- Add **identities**: a name, a role, a hashed token, a creation time, and a revocation time.
  Tokens are shown once at creation and stored only as a hash — a leaked database must not yield
  usable credentials.
- Three roles, matching the design's permission table: **admin** (approve capabilities, manage
  identities), **operator** (store secrets, invoke), **agent** (invoke only). The `runner` role
  arrives with the runner itself.
- Add an **audit record** for every attempt, successful or refused: who, when, what was
  attempted, against which resource, and the outcome. Refusals matter more than successes — they
  are what an injected agent produces.
- Add `sealbox identity create|list|revoke` and `sealbox audit` to the CLI.
- **Bootstrap**: with no identities in the database, the server accepts a deploy-time
  `SEALBOX_BOOTSTRAP_TOKEN` to create the first admin, once, within a bounded window. It is never
  logged.

## Capabilities

### New Capabilities

- `identity`: who a caller is, what role they hold, how they prove it, and how access is
  withdrawn — including how the first identity comes to exist.
- `audit`: the record of what was attempted, by whom, and whether it was allowed.

### Modified Capabilities

- `http-api`: the requirement "Business endpoints require authentication" currently describes a
  single shared token. It becomes authentication as a named identity, with authorisation by role.

## Impact

**Implements** the identity half of the MVP's item 2. **Blocks** grants, jobs, and the runner,
each of which needs a caller to attribute work to.

**Constrained by** ADR 0003 (agents invoke, humans approve — the role split exists to make that
enforceable), ADR 0009 (admin authentication is replaced later; do not build anything that
assumes an admin token is permanent), and ADR 0012 (no migration path for existing deployments).

**Code**
- `sealbox-server/src/api/auth.rs` — token comparison becomes identity lookup and role check
- `sealbox-server/src/repo/` — `identities` and `audit` tables, and their repositories
- `sealbox-server/src/api/handler/*` — each endpoint declares the role it requires
- `sealbox-server/src/config.rs` — `SEALBOX_AUTH_TOKEN` out, `SEALBOX_BOOTSTRAP_TOKEN` in
- `sealbox-cli/src/` — `identity` and `audit` commands; config carries an identity's token

**Interfaces** — every existing endpoint changes its authentication requirements. Existing
callers stop working, which is acceptable under ADR 0012.

**Security** — this is the change that makes an agent's authority narrower than a human's.
Getting the role check wrong in one handler undoes it, so authorisation is checked in one place
rather than per handler.
