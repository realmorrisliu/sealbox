# Built-in adapters first, scripts as the escape hatch — and sealbox always generates the values

A grant's implementation is either a **built-in adapter** or a **custom script**. Adapters are
preferred; scripts exist so that anything not covered is still possible.

```toml
[k8s-sync]
adapter = "kubernetes-secret"
runner  = "prod-cluster"
config  = { namespace = "utopia-system", name = "utopia-runtime-secret-bridge" }
secrets = { DATABASE_URL = "utopia/prod/database-url", OSS_ENDPOINT = "..." }
```

Three reasons adapters win where they apply:

1. **Nobody rewrites the same thing.** Without them, every user writes the same
   `kubectl create secret --from-env-file | kubectl apply` by hand. The recurring targets are
   few: Kubernetes Secrets, Postgres/MySQL roles, GitHub secrets, docker registry auth, SSH keys.
2. **Approval becomes reading a declaration instead of reading code.** The reviewer sees
   "kubernetes-secret, into utopia-system/xxx, using these four secrets" — there is nothing to
   audit line by line, which is the review that actually gets done.
3. **Adapters narrow capability; scripts do not.** A custom script can do anything its declared
   secrets permit — a script holding a kubeconfig could `delete ns prod`. The
   `kubernetes-secret` adapter can only write a Secret. That is a structural bound, not a
   convention.

## Not a plugin system

What is rejected is Vault's model: dynamically loaded engines, each with its own configuration
DSL and path semantics, so that learning cost multiplies. Adapters here are compiled into the
binary — no loading mechanism, no per-engine DSL, no version negotiation. The escape hatch is a
plain script, not a plugin API.

The cost of the alternative was measured on this infrastructure: everxyz/Utopia#695 retired ESO
and KMS, deleting 771 lines including a 379-line key-name verifier and a 162-line manifest
checker.

**Growth rule, to stop this becoming Vault:** an adapter is only built in once it would replace
**at least two scripts that actually exist**. Never write one for an imagined need — let a script
be written, let it be duplicated, then converge it.

MVP ships two: `kubernetes-secret` and `postgres-role`. They are exactly what the acceptance
scenarios need.

## Sealbox generates the values, either way

Neither adapters nor scripts produce secret material. The server generates the value and injects
it as `$SEALBOX_NEW`; the implementation's job is to make some upstream accept it, and to emit
whatever composed form should be stored (`--from-output`). Randomness stays in one audited place
instead of being reimplemented, badly, per script.

These are two orthogonal switches: where the value handed to the grant comes from (sealbox,
always), and what gets stored afterwards (the generated value, or the implementation's stdout).
Rotating a password uses the first alone; provisioning a database uses both.

## Who writes grants

Agents draft them; humans approve them. Requiring a human to author every grant from scratch would
restore exactly the adoption cost this project exists to avoid. With adapters, most drafts are
configuration, which an agent gets right by imitation from `examples/grants/`.

**Script bodies are stored in sealbox, never referenced by path.** A grant pointing at
`/opt/sealbox/scripts/x.sh` could be approved once and its file edited afterwards; what was
reviewed and what executes would differ. `sealbox grant add` ingests the content, and execution
materialises it as a `0600` temp file.

Approval is gated by the admin credential alone — every grant, no tiers. Removing approval was
considered and rejected outright: an agent that can approve its own grants can write
`command = ["sh", "-c", ...]`, approve it, and run it, which voids ADR 0003 and reduces the
broker to a shell holding every credential.
