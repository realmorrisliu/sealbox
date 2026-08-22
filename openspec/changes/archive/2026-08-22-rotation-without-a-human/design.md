# Design

## Where the human belongs

The principle this follows is recorded in [ADR 0013](../../../docs/adr/0013-automation-first.md):
automate first, reach for an agent only where judgement is genuinely needed, and keep a person for
the one thing that must not be automated — **irreversibly widening authority**.

Rotation does not widen anything. The grant that performs it was approved once, and every
execution afterwards is that same approved authority being used again. That is exactly the line:
approving `pg-provision` is the human act; running it the fortieth time is not.

## `rotate_after` is a declaration, not a timer

It is deliberately inert. The alternative — the server acting on it — is a scheduler, and a
scheduler brings retry policy, backoff, overlap, and calendars, which is a system rather than a
field.

Stored per version and **carried forward by a rotation**, or the policy would be lost the first
time it was honoured, which is the worst possible moment.

## Not the TTL

`--ttl` already exists and means *delete*. Using it as a rotation deadline would remove a
credential that production is still using, at exactly the moment it is most in use. They are
different fields, and the CLI says so where someone would reach for the wrong one.

## Overdue is a question, not a state

`--overdue` filters at read time from `updated_at + rotate_after`. No flag is stored, nothing
sweeps, and a secret becomes current again by being rotated — because the only thing that makes it
overdue is the timestamp that a rotation moves.
