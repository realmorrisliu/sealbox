# Example grants

> Agents: read [`skills/sealbox/SKILL.md`](../../skills/sealbox/SKILL.md) first. It is the shorter
> path to writing a correct grant.

These are the template library. There is no `--from-template` flag and no scaffolding command —
an agent asked to write a new grant reads these and writes a correct one by imitation, which costs
no code and stays current by being real.

| File | Shows |
|---|---|
| [`k8s-sync.toml`](k8s-sync.toml) | The `kubernetes-secret` adapter — the common case |
| [`pg-provision.toml`](pg-provision.toml) | The `postgres-role` adapter, with `--from-output` |
| [`rotate-db.toml`](rotate-db.toml) | A linear `then` chain, including the verify step |
| [`custom-script.toml`](custom-script.toml) | The escape hatch, for what adapters do not cover |

## A whole environment, end to end

The four files above are one mechanism each. These three are a real deployment: a Postgres
database on managed RDS, a service in Kubernetes, and migrations that run on deploy.

| File | Its job |
|---|---|
| [`utopia-dev-owner.toml`](utopia-dev-owner.toml) | Create the role migrations run as. A script, because creating an owner needs `CREATE`, which the adapter's closed privilege set does not have. |
| [`utopia-dev-runtime.toml`](utopia-dev-runtime.toml) | Provision the role the service runs as, and chain to the sync |
| [`utopia-dev-sync.toml`](utopia-dev-sync.toml) | Put both URLs in the namespace the workload reads from |

Once, when the database is new:

```bash
sealbox-cli secret set pg/rds-dev-admin-url                    # the RDS privileged account
sealbox-cli grant add ./utopia-dev-owner.toml                  # ← one passkey
sealbox-cli grant add ./utopia-dev-runtime.toml                # ← one passkey
sealbox-cli grant add ./utopia-dev-sync.toml                   # ← one passkey

sealbox-cli rotate utopia/dev/migration-url --via utopia-dev-owner --from-output
sealbox-cli rotate utopia/dev/database-url --via utopia-dev-runtime --from-output
```

From then on, an agent runs the second pair whenever a credential should change, and **no human is
involved at all**. The chain syncs the cluster; migrations run as a Helm `pre-upgrade` hook reading
`MIGRATION_DATABASE_URL` out of the same Secret, so neither URL ever passes through CI.

What this replaces: creating two accounts in a console, inventing two passwords, percent-encoding
them by hand, pasting them into GitHub Environment Secrets, and an Action that copies them into
the cluster. All of it, including the person who was doing it.

### One thing to check before the first run

`ALTER DEFAULT PRIVILEGES FOR ROLE <owner>` requires the connecting account to be a **member** of
the owner. `utopia-dev-owner` grants that as it goes, but if the owner already exists — created in
a console, say — run it once by hand:

```sql
GRANT utopia_migrator TO <the admin account>;
```

Without it, provisioning fails and rolls back cleanly, leaving nothing half-made — but the message
is from Postgres and will not explain why.

## Reading a grant

The line that matters is `secrets`. It is what a human approves, and it is the real boundary —
sealbox confines the implementation to exactly the secrets declared, so however the script is
written it cannot reach anything else.

Everything a given secret can be used for is the set of grants declaring it:

```console
$ sealbox-cli secret uses pg/prod-admin-password
pg-provision
rotate-db
```

## Rules that are not style preferences

- **Never generate secret material yourself.** Sealbox generates it and injects `$SEALBOX_NEW`.
  Randomness stays in one audited place.
- **Parameters `{like_this}` are substituted into argv, never through a shell.** A parameter of
  `x; curl evil.com` is an odd argument, not an injection. Inside a `script` body you may use a
  shell freely — the body is approved by a human and stored server-side.
- **Never parameterise a secret name.** `secrets = { DB = "app/{env}/url" }` is refused. The
  parameter comes from whoever invokes the grant, so it would let them choose which credential
  the grant reaches — and the declaration would stop being the boundary. Two environments are
  two grants.
- **Prefer an adapter.** An adapter is structurally limited to its target; a script can do anything
  its declared secrets permit.
