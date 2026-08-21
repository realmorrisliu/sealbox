## Context

See proposal.md — Why.

What exists today: `api/auth.rs` compares a bearer token against `config.auth_token` in a single
`route_layer` covering every business endpoint. Public routes are registered after that layer.
There is no notion of a caller, and no record of anything.

What this must not paint into a corner: [ADR 0009](../../../docs/adr/0009-admin-authenticates-with-passkeys.md)
replaces admin authentication with passkeys. Admin identities holding tokens is a temporary
state, so nothing may assume a credential is the only way an identity proves itself.

## Goals / Non-Goals

**Goals:**

- Each caller is a distinct identity whose authority is checked in one place.
- An agent's authority is strictly narrower than a human's, enforced rather than documented.
- Every attempt, including refusals, is attributable and recorded.
- A leaked database yields no usable credential.

**Non-Goals:**

- Passkeys, invites, join tokens. Those arrive with ADR 0009 and the runner.
- The `runner` role — added when there is a runner to hold it.
- Per-resource permissions. Roles are coarse on purpose; the fine-grained boundary in this design
  is the grant, not an ACL.
- Audit retention or rotation. The table only grows; revisit when that is a real problem rather
  than an anticipated one.

## Decisions

### Authorisation lives in the router, not in handlers

Endpoints are grouped by the role they require, and each group gets its own layer:

```rust
Router::new()
    .merge(admin_routes().route_layer(require_role(Role::Admin)))
    .merge(operator_routes().route_layer(require_role(Role::Operator)))
    .merge(agent_routes().route_layer(require_role(Role::Agent)))
    // public routes registered last, so no auth layer covers them
```

The spec requires that an endpoint added later be refused by default rather than exposed by
omission. Grouping gives that for free: a route not placed in a group is not in the router at
all, so it 404s. A per-handler check would give the opposite default — forget the line, and the
endpoint is open.

This also puts the ordering hazard in one visible place. `route_layer` applies only to routes
registered before it, which is exactly how the health probes ended up behind authentication in
the previous change.

*Alternative rejected:* an extractor each handler declares (`RequireRole<Admin>`). It reads well,
but a handler that omits it compiles and serves.

### Roles are ordered, and checked as a threshold

`Agent < Operator < Admin`. `require_role(X)` admits any role at or above X. Three roles with a
natural inclusion order do not need a permission matrix, and a matrix would invite per-resource
entries — the thing this design is deliberately not building.

When that stops being true, the answer is a grant, not a finer role.

### Tokens are hashed with SHA-256, not a password KDF

An identity's token is 256 bits from a CSPRNG, not a human-chosen password. Argon2 or bcrypt
exist to make guessing a low-entropy secret expensive; against a random 256-bit value, guessing
is already impossible, and a slow KDF would only add latency to every request.

Lookup is by hash, so it is a single indexed query rather than a scan comparing candidates.
Comparison is still constant-time to avoid leaking through timing.

Tokens carry a `sealbox_` prefix. It costs nothing and makes a leaked token recognisable to
secret scanners and to a human reading a config file.

*Alternative rejected:* storing tokens encrypted rather than hashed. Encryption implies the
ability to decrypt, and there is no operation that needs a token's plaintext after creation.

### Audit is written by middleware, not by handlers

The middleware knows the identity, the method, the path, and the outcome — which is precisely
what the spec requires, and it covers refusals, which never reach a handler at all.

Handlers cannot be relied on for this: a refused request has no handler, and a handler that
forgets to record leaves a silent gap exactly where an injected agent would produce evidence.

Resource names come from the path (`/v1/secrets/{key}`), which is where they already are.

*Alternative rejected:* recording in handlers for richer detail. Detail is worth less than
completeness here, and the completeness is what makes a refusal visible.

### A failed audit write fails the request

If the record cannot be written, the action does not happen.

The alternative is an action that occurred with no trace, which is the one outcome this
capability exists to prevent. The practical cost is low: audit and business data share one
database, so a failing audit write means the request was going to fail anyway.

### Bootstrap is a separate path, not a special identity

With zero identities, one endpoint accepts `SEALBOX_BOOTSTRAP_TOKEN` and creates the first admin.
It is not modelled as an identity with a magic name, because such a row would persist and have to
be defended against afterwards.

Three conditions, all required: no identity exists, the token matches, and the server started
less than 30 minutes ago. The last bounds exposure when a token is left in the environment after
use — which will happen.

The first admin's creation is itself audited, from an empty trail.

## Risks / Trade-offs

- **Getting a route into the wrong group silently widens access** → Grouping makes the intended
  role visible at the routing table rather than buried in handlers, and tests assert the matrix
  per endpoint rather than per handler.
- **Admin identities hold tokens, which ADR 0009 removes** → The role check does not care how an
  identity authenticated. Passkeys replace how a credential is presented, not what it resolves to.
- **The audit table grows without bound** → Accepted. It is append-only and small per row;
  retention becomes a change when the size is measurable rather than imagined.
- **A stolen agent token is usable until revoked** → What revocation is for, and why the audit
  trail records refusals. Short-lived credentials are a later change, not a MVP requirement.
- **Every existing caller breaks** → Intended, per ADR 0012.

## Migration Plan

None. `SEALBOX_AUTH_TOKEN` is removed; existing deployments do not exist (ADR 0012). Starting a
server with an empty database and a bootstrap token yields a first admin, and every other
identity is created from there.

**Rollback:** revert the binary. The added tables are ignored by earlier code.

## Open Questions

None.
