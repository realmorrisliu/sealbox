# Rotation without a human

## Why

Two things stand between a rotation and nobody having to do it, and neither is a security
boundary.

**An agent is refused.** `rotate` requires the operator role. The reasoning recorded in the test
was that "an agent may run a grant that reads secrets; changing one is a different thing" — but
under an approved grant the two have the same reach. Rotation cannot exfiltrate (the value is
generated server-side and returned to nobody), the path was signed for by a human,
create-new-then-drop-old leaves the previous credential working, and every step is audited. An
agent that may `run` a script grant which creates a database role can already do what `rotate`
does; the line is drawn at the adapter's implementation rather than at authority, which makes it
an inconsistency rather than a conservative choice.

**Nothing knows what is due.** A secret records when it last changed and nothing about when it
should change again. So the schedule ends up in a CronJob in each cluster, next to an operator
token in a Secret — a long-lived credential a person maintains, to run an operation the same
person was told they had to run.

## What changes

- `rotate` admits the agent role, matching what the documentation has said all along.
- A secret may declare `rotate_after`, and a rotation carries it to the new version.
- `secret list --overdue` answers which secrets are past it.

## What is deliberately not built

**A scheduler.** `rotate_after` is a declaration and nothing acts on it: sealbox stores the policy
and answers the question, and whatever runs on a timer stays outside. If per-cluster CronJobs turn
out to be the shape everyone builds, the natural home is the **runner** — already long-polling,
already the only executor, and it would need no new credential — but that reverses a recorded
decision and should wait until there are enough periodically-rotated secrets to prove it.

**An agent that decides what is stale.** Judging `updated_at + rotate_after < now` needs no
judgement. Reaching for an agent where a flag will do is a downgrade, not automation: an agent is
non-deterministic, costs tokens, and can be talked into things.
