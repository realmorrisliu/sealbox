## Why

`gen` can produce a value, but nothing can tell the system that already holds the old one. A
generated database password nobody told Postgres about is a random string nothing accepts — so
generation, on its own, is not usable for the credentials that most need replacing.

Rotation is the operation that closes that: produce a new value, get some upstream to accept it,
and commit the new value **only if that worked**. If the upstream push fails, the old value must
still be the current one — otherwise a failed rotation is worse than no rotation, because the
stored credential no longer matches reality and nothing says so.

It is also what makes an agent able to *maintain* credentials rather than only use them. An agent
can rotate a production database password without ever learning either the old one or the new.

## What Changes

- Add `sealbox rotate <secret> --via <grant> [--from-output] [key=value ...]`.
- The server generates the new value and provides it to the grant as `$SEALBOX_NEW`. **An
  implementation never produces secret material** ([ADR 0007](../../../docs/adr/0007-adapters-first-scripts-as-escape-hatch.md)):
  randomness stays in one audited place rather than being reimplemented, badly, per script.
- The new value is stored as a **pending version** — encrypted like any other, but invisible to
  reads and listings — and becomes current only when the grant succeeds. A failure removes it,
  leaving the previous version current and unchanged.
- `--from-output` stores what the grant printed instead of the generated value, for values that
  are *composed* rather than raw: a `DATABASE_URL` with a percent-encoded password, or a
  credential only an upstream can issue.
- A captured value is **never written to the job record**. Job output is stored in plain text for
  the caller to read; a captured value goes straight into the envelope.
- Rotation reuses the job queue and the runner, so a rotation runs in the same place, under the
  same constraints, as anything else.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret`: gains pending versions and the rule that a value can be replaced only through a
  successful grant. The existing requirement that writing creates a new version needs to say what
  happens to a version whose rotation failed.
- `job`: gains the generated value handed to an implementation, and the captured value returned
  from one — including that the captured value does not pass through the job record.

## Impact

**Implements** MVP item 7. **Unblocks** the `postgres-role` adapter, whose reason for existing is
create-new-then-drop-old rotation.

**Constrained by** ADR 0011 (dual credentials, a linear chain, verification before the old
credential is dropped), ADR 0007 (sealbox generates, implementations do not), and the
`secret-encryption` requirement that a value never appears in diagnostics — of which the job
record is one.

**Code**
- `sealbox-server/src/repo/` — a pending flag on secret versions, and commit/discard
- `sealbox-server/src/api/handler/` — the rotate endpoint, and `$SEALBOX_NEW` in a claim
- `sealbox-cli/src/` — `rotate`, and capture in the runner

**Security** — the failure mode this exists to prevent is a stored credential that silently
disagrees with reality. The other is a captured value leaking through a field meant for
human-readable output; both are addressed by construction rather than by care.
