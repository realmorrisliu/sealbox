# Rotation creates a second credential and runs a linear chain

Rotating a credential never mutates the existing one. The adapter creates a **new** credential
alongside the old, the new value is propagated, the result is verified, and only then is the old
one removed.

`ALTER USER ... PASSWORD` has a window in which the credential in the database and the credential
in the cluster disagree, and every request in that window fails. Dual credentials remove the
window: at each step, at least one working credential exists. This is what Vault and AWS Secrets
Manager both do, and it is a requirement on the adapter's implementation, not advice to whoever
writes the grant.

```
create app_v2 → sync Secret → restart workload → verify health → drop app_v1
```

## Orchestration is a linear chain on the server

```toml
[rotate-utopia-db]
adapter = "postgres-role"
then    = ["k8s-sync", "k8s-restart", "verify-health"]
```

Not left to the agent: an agent can be interrupted, distracted, or injected between steps, and a
half-finished rotation is exactly the state nobody wants to be in. The server runs the chain,
stops at the first failure, and records which step failed.

**Explicitly not implemented:** retries, rollback, parallelism, conditional branching,
scheduling. Linear, stop-on-failure, and that is all. It is a reduced form of AWS's four rotation
stages (`createSecret` / `setSecret` / `testSecret` / `finishSecret`); the verification step is
the one people skip and the one that prevents deleting a working credential in favour of a broken
one.

## Consequences

A failed chain leaves both credentials alive and the old one still in service — recoverable, and
visible in audit. Adapters that support rotation must implement create-new-then-drop-old; one
that can only mutate in place cannot be a rotation target.
