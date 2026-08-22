## Context

See proposal.md — Why.

Grants exist, are validated at approval, and declare a runner by name. Nothing resolves that name
and nothing executes. Roles are ordered and compared as a threshold, which is exactly what a
runner does not fit.

## Goals / Non-Goals

**Goals:**

- A grant executes, in the one place where that is acceptable.
- A runner receives only what its claimed job needs, and cannot ask for more.
- Plaintext leaves the server only into a runner, and only for the duration of a job.
- `sealbox run` returns an exit status and output, never a value.

**Non-Goals:**

- Adapter implementations. A `script` grant exercises every part of this; adapters are a second
  implementation of the same interface.
- Retries, scheduling, priorities, parallelism, dead-letter handling. Linear, claim-once,
  stop-on-failure ([ADR 0011](../../../docs/adr/0011-rotation-uses-dual-credentials-and-a-linear-chain.md)).
- Streaming output while a job runs. The result arrives when it is done.
- More than one runner claiming from the same queue for redundancy. One name, one runner, for now.

## Decisions

### The runner role is disjoint, and needs a different gate

`require_role` admits anything at or above a threshold. A runner is not "below agent" or "above"
anything — it may do one thing no other role may, and nothing else that any of them may.

So `Role::Runner` sits below `Agent` in the ordering, which makes every existing threshold gate
refuse it for free, and job claiming gets its own gate that matches the role **exactly**. Admin
does not inherit it: being the most privileged identity does not make you the machine the job was
addressed to.

*Alternative rejected:* a permission set per role instead of an order. Three roles with a natural
inclusion order plus one exception is not a matrix's worth of complexity — and a matrix invites
per-resource entries, which is the thing grants exist to be instead of.

### Claiming is one atomic UPDATE

```sql
UPDATE jobs SET status = 'claimed', claimed_at = ?, claimed_by = ?
WHERE id = (SELECT id FROM jobs WHERE status = 'pending' AND runner = ? ORDER BY id LIMIT 1)
```

The write itself decides the winner, so two runners polling at once cannot both get the same job.
A read-then-write would need a transaction and a retry loop to say the same thing less clearly.

### Long polling is a sleep loop, deliberately

The claim endpoint retries the statement every 200ms for up to 30 seconds before returning empty.
No channels, no notification bus, no shared wakeup state.

`sealbox run` is synchronous — someone is watching — so a plain 5-second poll would make every
invocation feel broken. This costs one cheap indexed query per runner per 200ms, which at the
scale of one runner is nothing.

> ponytail: polling loop. If the number of runners ever makes this measurable, the fix is a
> notify channel keyed by runner name, not a longer interval.

### A claim carries values, not references

The claim response contains the plaintext of the declared secrets. The runner cannot ask for a
secret by name — there is no endpoint that does that, for any role.

This is the whole reason the runner exists and the reason it is placed inside the infrastructure
rather than on a laptop: plaintext exists there, for one job, and nowhere else outside the server.

### Files are created per job and removed in a guard, not at the end

File-shaped secrets are written to a `0600` temp directory removed when the job's scope ends,
including on panic or early return — not with a cleanup call at the bottom of the happy path,
which is exactly the line that gets skipped when an error path is added later.

### Chains are driven by the server, not the runner

When a job succeeds and its grant declares a chain, the server queues the next one. The runner
knows nothing about chains.

A runner that drove the chain would have to be trusted to keep going, and would keep going
without supervision if it were compromised. The server already decides what may run; it decides
what runs next.

### The audit trail records jobs, not just requests

A job's submission, claim, and result are three audit entries. The existing middleware already
records the HTTP requests, but "who ran what, on which runner, and what happened" is the question
the trail exists to answer, and it should not have to be reconstructed from three URL paths.

## Risks / Trade-offs

- **A compromised runner sees the plaintext flowing through it** → Bounded to jobs addressed to
  it and the secrets those grants declare. Narrowing further means a second runner with a
  narrower ServiceAccount, which is the recorded answer rather than a policy engine.
- **A stuck job blocks nothing but itself** → Claim-once plus a timeout means one lost job, not a
  stalled queue.
- **No retries means transient failures surface as failures** → Intended. A caller can resubmit;
  the system cannot know whether re-running is safe.
- **Long polling holds a connection per runner** → One runner, one connection. Revisit when there
  are many.
- **`sealbox run` blocks until the job finishes** → Matches what it is for. A long-running grant
  needs an async form; nothing needs that yet.

## Migration Plan

None; new table, new endpoints, one new role value. Existing identities are unaffected.

**Rollback:** revert the binary. An unused `jobs` table is inert.

## Open Questions

None.
