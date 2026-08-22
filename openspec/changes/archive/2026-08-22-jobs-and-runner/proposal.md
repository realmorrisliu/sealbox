## Why

Grants can be approved but not executed. Everything before this change built the constraints; this
one makes them do work.

The shape is forced by two facts that have nothing to do with preference
([ADR 0008](../../../docs/adr/0008-runner-is-the-only-executor.md)):

**A hosted server cannot reach your infrastructure.** An RDS instance inside a VPC is not
reachable from a Fly.io machine, and giving it a route would mean exposing production to the
public internet. So the server cannot execute.

**An agent's own machine must not hold plaintext.** If the CLI decrypted and ran things locally,
the secret would exist on a host the agent controls, and it could observe the execution it
triggered. So the CLI cannot execute either.

What is left is a third place: a **runner** inside your infrastructure, which polls the server
**outbound** — so your cluster opens no inbound port, needs no Ingress, and no public endpoint.
It is the only place a grant runs and the only place plaintext exists outside the server.

## What Changes

- Add a **job queue**: one requested execution of a grant, its parameters, the runner it is
  addressed to, and its result. Claim-and-report, with a timeout that marks abandoned jobs failed.
  **No automatic retries** — grants are not necessarily idempotent, and silently re-running a
  `CREATE USER` or a deployment is worse than failing.
- Add the **`runner` role**, whose permissions are *disjoint* from every other rather than
  ordered: it may claim jobs addressed to it and report results, and may do nothing else. It
  cannot invoke a grant, list secrets, or read the audit trail; no other role can claim a job.
- Add `sealbox runner --name <name>`: long-polls, claims, executes, reports.
- **Three injection forms**, because real consumers need all three: an environment variable, a
  single `0600` temp file whose path is substituted into argv, and an env-file rendering several
  secrets at once for `--from-env-file` style consumers.
- Execution is **argv, never a shell**. A parameter of `x; curl evil.com` is an odd argument, not
  an injection.
- Add `sealbox run <grant> [key=value ...]`: submit, wait, print the result — never plaintext.
- Chains run on the server, in order, stopping at the first failure.

Explicitly **not** in this change: the two adapter implementations. A grant with a `script` runs
end to end here, which exercises every part of the machinery; adapters are a second
implementation of the same interface and land next.

## Capabilities

### New Capabilities

- `job`: what a requested execution is, how it is claimed and reported, what a runner may see,
  and what happens when one is abandoned.

### Modified Capabilities

- `identity`: the role model gains `runner`, which is disjoint rather than ordered. The existing
  requirement describes three ordered roles and needs to say how a fourth that fits no threshold
  is handled.

## Impact

**Implements** MVP item 5, and item 6's `run`. **Blocks** `adapters` and `rotation`.

**Constrained by** ADR 0008 (the runner is the only executor; the CLI is a remote control),
ADR 0003 (an agent supplies a grant name and parameters, never a command), and ADR 0011 (chains
are linear and stop on failure; no retries, no branching).

**Code**
- `sealbox-server/src/repo/` — a `jobs` table and its repository
- `sealbox-server/src/api/handler/` — submit, claim, report, and poll endpoints
- `sealbox-server/src/api/auth.rs` — a disjoint gate for the runner role
- `sealbox-cli/src/` — `run`, and the `runner` subcommand that executes

**Security** — this is the change that puts plaintext somewhere other than the server. What
bounds it: a runner receives only the secrets the claimed grant declares, only for jobs addressed
to it, and cannot ask for anything. Its row in the permission table is disjoint from every other
— it takes what it is given and reports back.
