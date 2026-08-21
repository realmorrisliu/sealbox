# The runner is the only executor; the CLI is a remote control

Grants are executed by a `sealbox runner` process that long-polls the server for work. The CLI
never executes a grant and never receives a secret's plaintext — it submits a job and displays the
result.

Two forces produce this. The server is hosted (Fly.io) and cannot reach an RDS instance inside a
VPC; giving it that route would mean exposing production to the public internet. And executing on
the agent's own machine puts plaintext on a host the agent controls.

A runner inside the cluster solves both. It reaches the VPC because it is in it, and it **polls
outbound**, so the cluster needs no inbound port and no public endpoint. Push delivery was
rejected for exactly that reason: it would require the runner to be reachable from outside.

## One execution path, not two

"Simple grants run locally, VPC ones go through a runner" was rejected. Two paths mean two
security models, and the weaker one silently becomes the default. There is one path: runners
execute. Running something locally means running a local runner —

```bash
sealbox runner --name laptop
sealbox runner --name prod-cluster   # a Deployment in the cluster
```

— the same binary and the same subcommand. A grant declares which runner it belongs to.

## Consequences

**Two of the three known holes close.** An agent's machine never holds plaintext, so it cannot
observe an execution it triggered; and results are reported by the runner, so an agent cannot
fabricate a `--from-output` value. The third — an agent stealing a human's admin token — was
still open when this was written, and is closed separately by ADR 0009, which removes the stored
admin credential entirely.

**The kubeconfig disappears as a stored credential.** An in-cluster runner uses its
ServiceAccount, so `kubectl` works with no credential to store, distribute, or rotate.

**A new high-value target exists.** The runner holds plaintext for the grants it executes.
Compromising the cluster means compromising the credentials that flow through it. It is confined
to jobs addressed to it and cannot enumerate or read arbitrary secrets, so the exposure is the
grants it runs — not the store.

**Availability is coupled to the runner.** If it is down, nothing executes. Accepted: the failure
is visible and local, and a second runner is one more Deployment.

**A job queue enters the MVP.** Kept deliberately small — one table, claim-and-report, a timeout
that marks abandoned jobs failed. **No automatic retries**: grants are not necessarily idempotent,
and silently re-running a `CREATE USER` or a deployment is worse than failing.
