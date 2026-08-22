## Context

See proposal.md — Why.

What exists: values can be supplied or generated; grants declare secrets and are executed by a
runner; jobs carry parameters and return an exit status and output. What does not: any way to
make a new value conditional on something succeeding.

Two facts shape the whole design:

- The new value must exist somewhere between being generated and being committed. Wherever that
  is, it is a plaintext credential in storage.
- Job output is stored in the clear, because the caller reads it. A captured value must not travel
  that way.

## Goals / Non-Goals

**Goals:**

- A failed rotation leaves the stored credential matching reality.
- The new value is never chosen by, or returned to, the caller.
- A captured value reaches storage without passing through anything readable.

**Non-Goals:**

- Scheduled or automatic rotation. A rotation is requested; nothing decides on its own that now
  is the time.
- Rolling back a *successful* rotation. Once upstream has accepted a value, undoing it is a new
  rotation with its own grant — not a stored copy of what came before.
- Rotating more than one secret in one operation. A chain covers the cases that need it.

## Decisions

### The pending version is a real version, marked

The new value is written through the same path as any other — enveloped, versioned — with a
`pending` flag. Reads, listings, and claims exclude pending versions. Success clears the flag;
failure deletes the row.

The alternative is holding it somewhere else until commit: in the job row, in memory, in a
separate table. In the job row it would be a plaintext credential in a table that exists to be
read. In memory it would not survive a restart, and a rotation that lost its new value after
upstream accepted it is precisely the disagreement this feature exists to prevent. A separate
table would be the secrets table with a different name and its own encryption path to get wrong.

*Consequence, accepted:* a failed rotation consumes a version number. Version 3 may not exist
while 2 and 4 do. That is visible and explicable; the spec only requires that the numbering not
imply a value is missing, and a deleted pending row leaves nothing to look for.

### The generated value is injected as a declared secret

`$SEALBOX_NEW` is added to the claim's secrets map, so the runner materialises it exactly like
any other — environment variable, and part of the env-file. The implementation cannot tell it
apart, which is the point: there is nothing special for a script author to get right.

### Capture is a separate field, all the way down

The runner reports `{ exit_code, output, captured }`. `output` is stored on the job. `captured`
is encrypted into the pending version and then dropped.

Not "parse the value out of stdout" — that would make every rotation's correctness depend on an
implementation not printing anything else, and a stray log line would silently store the wrong
credential. The runner separates the two because the grant tells it to: a capturing rotation
takes the implementation's stdout as the value and requires it to print nothing else.

An empty capture fails the rotation. Storing an empty credential because a script forgot to print
is the same failure mode as storing the wrong one.

### Rotation reuses the job queue rather than bypassing it

A rotation submits an ordinary job with two extra properties: the generated value goes in, and
the outcome decides a commit. Same queue, same runner, same audit trail, same permission gate.

A separate execution path would need its own answers to claiming, timeouts, and abandonment —
all of which exist, and none of which would be improved by being written twice.

### Rotation requires the operator role

Rotating changes a stored value, which is what `set` requires the operator role for. An agent can
*run* a grant that reads secrets; changing one is a different thing.

*Note:* the design has an agent provisioning credentials, which means giving it a rotation. That
is an operator identity for that agent — a deliberate grant of "may change stored values" — not
a widening of what `agent` means.

## Risks / Trade-offs

- **An abandoned rotation leaves a pending version forever** → The sweeper that fails abandoned
  jobs discards the pending version with them. A pending row with no live job is unreachable by
  every read path regardless.
- **Upstream accepts the value but the runner dies before reporting** → The worst case, and
  unavoidable without a distributed transaction. The stored value stays old, the audit trail shows
  a rotation that never reported, and the operator can see both. This is why `postgres-role` will
  create a second credential rather than mutate one (ADR 0011): both work until one is dropped.
- **A capturing grant that prints diagnostics corrupts the secret** → Documented in the grant
  format: a capturing implementation prints the value and nothing else. Diagnostics go to stderr.
- **A version number can be skipped** → Visible, explicable, and cheaper than renumbering.

## Migration Plan

The `secrets` table gains a `pending` column defaulting to 0, so every existing row is current.
No other change.

**Rollback:** revert the binary. A leftover pending row would become visible to an older build,
so discard pending rows before rolling back.

## Open Questions

None.
