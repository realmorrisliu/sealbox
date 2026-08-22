## Context

See proposal.md — Why.

What exists: identities with three ordered roles and a role gate applied per route group; secrets
that can be supplied or generated; an audit trail that records every attempt. What does not
exist: anywhere to execute anything.

## Goals / Non-Goals

**Goals:**

- A grant is a durable, reviewable statement of what may be done with which secrets.
- Approval is a real gate, enforced by role, and what a human reads is short enough to actually
  read.
- Everything checkable is checked when the human is present, not when the job runs.
- `--uses` answers, exhaustively, what a credential can do here.

**Non-Goals:**

- Executing a grant, materialising secrets, or implementing adapters. Those belong with the
  runner; there is nowhere to run them until it exists.
- Editing a grant. Replacing one is `rm` then `add`, which forces the declaration past a human
  again — the point of the gate.
- Per-grant identity restrictions ("only this agent may run this"). Roles are the coarse
  boundary and the grant is the fine one; a third axis needs a use case first.

## Decisions

### Validate at creation, not at execution

Every check that can be made when the grant is created — the secrets exist, the adapter is
known, the chain resolves and does not cycle — is made there.

A human is present at creation and can fix the problem. At execution nobody is, and the failure
lands mid-operation, possibly between steps of a chain. The same check costs the same either way;
only the moment differs, and one moment is much better than the other.

The trade is that a grant can be invalidated afterwards by deleting a secret it declares. That is
visible in `--uses` before the deletion, and the runner will report it plainly when it happens.

### Adapters are recognised by name now, implemented later

The set of adapter names is fixed in code, and a grant naming an unknown one is refused at
creation. The implementations arrive with the runner.

This is deliberately not a registry or a plugin lookup: adapters are compiled in
([ADR 0007](../../../docs/adr/0007-adapters-first-scripts-as-escape-hatch.md)), so the known set
is a constant, and adding one is a code change with a review attached.

### Chains are validated by walking, not by counting

Cycle detection walks the graph from the new grant and refuses if it reaches a grant already on
the path. A depth limit would be cheaper and would also catch cycles, but it would refuse a long
legitimate chain with a message about depth — describing the symptom rather than the mistake.

Worth noting how a cycle can arise at all: immutability plus creation-time validation means a
new grant can only chain to grants that already exist, and those cannot be edited to point back.
A cycle therefore requires `rm` followed by `add` — create B, create A chaining to B, remove B,
recreate B chaining to A. Rare, but reachable, and the check is cheap at this scale.

Chains are linear and stop on failure ([ADR 0011](../../../docs/adr/0011-rotation-uses-dual-credentials-and-a-linear-chain.md)):
no retries, no branching, no parallelism. That constraint lives in the runner; what lives here is
refusing a shape the runner could not safely execute.

### Grants are immutable once created

There is no update. Changing a grant means removing it and creating the replacement, which puts
the new declaration in front of a human exactly as the first one was.

An update endpoint would be the natural place for a capability to widen quietly — a `secrets`
list gaining one entry is a one-line diff that reads like nothing.

### The client parses TOML; the server takes JSON

Grant files are TOML because a human writes and reads them, and `secrets = { … }` is legible in a
way JSON is not. The wire format stays JSON like every other endpoint.

Parsing in the client also means a malformed file fails locally, with the file in hand.

### `--uses` is a filter on grants, not an index on secrets

Implemented as a query over grants' declared secrets rather than a maintained reverse index.
There will be tens of grants, and a stale index that disagrees with the grants themselves would
be worse than a scan — this answer is one people will act on.

## Risks / Trade-offs

- **A reviewer approves without reading** → What is put in front of them is the declaration, not
  the script: sealbox confines an implementation to exactly the secrets it declares, so the
  security-relevant question is one line long. Accepted and stated in ADR 0007.
- **A grant can be invalidated later by deleting a secret** → Visible through `--uses` beforehand;
  the runner reports it plainly. Blocking the deletion would let a stale grant pin a secret
  forever.
- **Immutability makes small edits tedious** → Intended. The tedium is the gate working.
- **Approval is by admin token until passkeys land** → Recorded in ADR 0009. The role check does
  not care how the identity authenticated.

## Migration Plan

None; new tables and endpoints only. **Rollback:** revert the binary. An unused `grants` table is
inert.

## Open Questions

None.
