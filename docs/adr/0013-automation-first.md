# Automate first; a human only widens authority

The rule for deciding whether a step needs a person:

1. **Automate it.** A deterministic rule beats everything else.
2. **If judgement is genuinely needed, an agent.** Not before — an agent is non-deterministic,
   costs tokens, and can be talked into things. Reaching for one where a flag would do is a
   downgrade dressed as automation.
3. **A human only where authority is widened irreversibly.** Approving a grant, creating an
   identity, enrolling an admin authenticator.

The measure of success: a person touches sealbox **once at setup, and once per new capability**.
Anything more often is the design leaking.

## Why the third rule is not negotiable

Everything sealbox is worth rests on one human act: someone signs for a capability before it
exists. Push "reduce human intervention" all the way and that act is the last thing left to
optimise away — at which point the product is a secret store with extra steps.

So the boundary is not *how often* a person is involved but *what for*. Using an approved grant is
not the moment authority is granted; approving it was. That is the line, and it is why a rotation —
which invokes a grant a human already signed for, with a value the server generates and returns to
nobody — needs no person, while adding the grant does.

## What this decided, immediately

**Rotation stopped being an operator's privilege.** It had required a role above `agent`, on the
reasoning that "running a grant that reads secrets and changing one are different things". Under an
approved grant they are not: an agent that may run a script grant which creates a database role can
already do what the rotation adapter does. The line was drawn at the adapter's implementation
rather than at authority — an inconsistency, not a conservative choice.

**`rotate_after` records a policy and nothing acts on it.** Where the knowledge lives matters: with
no field for it, every deployment ends up with the rotation schedule in a CronJob next to a
long-lived operator token, and "how old is this credential" becomes a question sealbox cannot
answer. It answers it now, and still runs no scheduler.

**No agent was built to decide what is stale.** `updated_at + rotate_after < now` needs no
judgement. It is a flag.

## What was rejected

**Scheduling inside sealbox.** A scheduler brings retries, backoff, overlap, and calendars — a
system rather than a field. If per-cluster cron jobs turn out to be what everyone writes, the
natural home is the **runner**: it already long-polls, it is already the only executor, and it
would need no new credential. That reverses a recorded decision, so it waits for evidence, on the
same growth rule the adapters use.

**Automating the master-key backup.** Nothing may hand out the master key, including to a script.
The one-time human step stays, and [ADR 0010](0010-recovery-via-keypair-not-a-copied-key.md)'s
ceremony is how it stops being a manual `cat`.

## Consequence

The remaining human toil is now visible and each item is a decision rather than an accident: the
recovery ceremony (ADR 0010, unbuilt) and runner join tokens (unbuilt) are the two places a person
still does work that no rule says they must.
