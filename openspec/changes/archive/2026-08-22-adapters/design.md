## Context

See proposal.md — Why.

Grants accept `adapter = "kubernetes-secret"` or `"postgres-role"` and validate the name; the
runner refuses both at execution. Everything else — claiming, injection, capture, rotation — is
built and exercised by script grants.

## Goals / Non-Goals

**Goals:**

- Both named adapters run.
- An adapter's configuration cannot make it do something else.
- What a human approves has no code in it.

**Non-Goals:**

- More adapters. The growth rule stands: one is built in only once it would replace **two scripts
  that actually exist** (ADR 0007). These two exist because the acceptance scenario needs them.
- Covering every use of Kubernetes Secrets or Postgres roles. Anything these do not cover is a
  script — that is what the escape hatch is for, and a general adapter is a script with worse
  ergonomics.
- Dropping the old Postgres role. That is a separate grant, run after something has verified the
  new one works (ADR 0011).

## Decisions

### Shell out to `kubectl` and `psql` rather than linking clients

A Kubernetes client crate is a large dependency tree for one call, and the runner already lives
where `kubectl` does — in the cluster, with a ServiceAccount `kubectl` picks up on its own.
`psql` likewise.

The security property does not come from the transport. It comes from the adapter constructing
the argv: the configuration supplies a namespace and a name, and the verb and resource kind are
fixed in code. A caller cannot reach a different operation through a field that only ever becomes
`--namespace`.

*Alternative rejected:* `kube-rs`. It would replace an external binary with about a hundred
crates, and would not narrow what the adapter can do by one bit.

### Configuration is a typed struct per adapter, deny_unknown_fields

Each adapter defines its settings as a struct, validated at grant creation. An unknown field is
refused rather than ignored — a typo in `namespace` that silently wrote to `default` would be
found in production, by someone who did not make it.

This is also where the "cannot widen" property is enforced mechanically rather than by care: for
a field to make the adapter do something else, someone would have to add it to the struct, and
that is a code change with a review attached.

### `postgres-role` names roles by prefix and a serial

Configuration gives a prefix; the adapter creates `prefix_1`, then `prefix_2`, and so on, picking
the next number by looking at what exists.

The alternative — put the role name in the configuration — would mean the second rotation had to
change the grant, which is immutable, so it would mean a new grant per rotation. A prefix makes
the grant stable across rotations, which is what makes it approvable once.

The old role is left in place. Dropping it belongs to a later grant that runs after something has
verified the new one works; an adapter that dropped the predecessor itself would remove the
credential that is still in production at that moment.

### Privileges are a closed set

`CONNECT`, `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `USAGE`. Anything else is refused at grant
creation.

Not because the others are dangerous in themselves, but because an open set would have to be
interpolated into SQL, and a field interpolated into SQL is a field that can carry SQL. A closed
set is matched against constants and never concatenated from input.

Grants beyond this set are a script.

### The k8s Secret is replaced, not merged

`create --dry-run=client -o yaml | apply -f -` replaces the Secret's contents. So removing a
secret from the grant removes it from the cluster, which is what someone editing the grant
expects — a merge would leave the old key behind and the removal would appear to have worked.

## Risks / Trade-offs

- **The runner's image must carry `kubectl` and `psql`** → Documented as a requirement of the
  runner image. A missing tool fails the job with a clear message rather than a confusing one.
- **`postgres-role` leaves old roles behind** → Deliberate; cleaning up is a separate grant. The
  cost of the alternative is dropping a credential that is still in use.
- **Scanning for the next serial races if two rotations run at once** → Two rotations of the same
  secret at once is already incoherent — both would create a pending version. Not defended
  against here; the second fails when its role name collides, which is a clear failure rather
  than a wrong one.
- **A closed privilege set will not fit someone** → It fits the acceptance scenario. Anything else
  is a script, which is the escape hatch working as intended.

## Migration Plan

None. Existing grants naming these adapters begin working; nothing else changes.

## Open Questions

None.
