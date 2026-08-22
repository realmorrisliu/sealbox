## Why

`kubernetes-secret` and `postgres-role` are accepted at approval and refused at execution. A
grant that names one is a grant that cannot run.

They are worth having for a reason that is easy to state wrongly. It is not that they save
typing — a script does the same work in ten lines. It is that **a script can do anything its
declared secrets permit, and an adapter cannot**
([ADR 0007](../../../docs/adr/0007-adapters-first-scripts-as-escape-hatch.md)). A script holding
a kubeconfig could `delete ns prod`; the `kubernetes-secret` adapter can write one Secret and
nothing else. That difference is the whole point, and it survives only if the adapter's
configuration is *structured parameters* rather than a place to put a command.

The second reason follows: what a human approves becomes a declaration with no code in it at all.
Judging whether a shell script is safe is a hard cognitive task, and that kind of review decays
into a glance. Judging `adapter = "kubernetes-secret"` plus a namespace does not.

## What Changes

- Implement **`kubernetes-secret`**: write the grant's declared secrets into one named Kubernetes
  Secret, using the runner's own ServiceAccount. Configuration is a namespace and a name —
  nothing that could name a different resource kind or a different verb.
- Implement **`postgres-role`**: create a role with the new value as its password, grant it a
  fixed set of privileges, and emit a connection URL. Configuration is a host, a database, a role
  prefix, and a list of privileges drawn from a closed set.
- `postgres-role` **creates a new role rather than changing an existing one's password**
  ([ADR 0011](../../../docs/adr/0011-rotation-uses-dual-credentials-and-a-linear-chain.md)):
  mutating in place has a window in which the credential in the database and the credential in
  the cluster disagree, and every request in it fails. The old role is dropped by a separate,
  later grant — after something has verified the new one works.
- Both are executed by the runner, which is where the network access is.

## Capabilities

### New Capabilities

- `adapter`: what a built-in implementation is, what bounds it, and what the two shipped ones do.

### Modified Capabilities

None. `grant` already says an adapter must be recognised at creation; this makes recognition mean
something.

## Impact

**Completes** MVP item 4.

**Constrained by** ADR 0007 (adapters narrow capability; scripts do not, which is why the escape
hatch stays), ADR 0011 (create-new-then-drop-old), and the `job` requirement that a runner
receives only the secrets its grant declares.

**Code**
- `sealbox-cli/src/commands/runner_commands.rs` — the two implementations, in the runner
- `sealbox-server/src/api/handler/grant.rs` — validate each adapter's configuration at approval,
  since a namespace that does not exist is better discovered while a human is present

**Security** — an adapter's configuration must not be able to widen what it does. A field that
takes SQL, a command, a resource kind, or a verb would turn the adapter back into a script with
extra steps, and the review it is supposed to make trivial would go back to being a code review.

**Dependencies** — both shell out to a tool the runner's image is expected to carry (`kubectl`,
`psql`) rather than linking a client library. A Kubernetes client crate is a large dependency
tree for one call, and the runner already lives where those tools do.
