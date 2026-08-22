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
