# Configuration

> **Partly implemented.** The server section's `SEALBOX_STORE_PATH`, `SEALBOX_LISTEN_ADDR`, and
> `SEALBOX_MASTER_KEY_PATH` work today, as does `SEALBOX_AUTH_TOKEN` — a single static token that
> identities and passkeys will replace. Everything about the runner, passkeys, invites, and
> `SEALBOX_PUBLIC_URL` describes the target and does not exist yet. See
> [`README.md`](../README.md).

Three things are configured separately, because they are trusted differently: the **server** holds
everything, the **runner** holds plaintext transiently, and the **CLI** holds nothing but a
pointer and an identity.

## Server

Environment variables. On Fly.io, non-secret values go in `fly.toml`, secrets via `fly secrets`.

| Variable | Required | Meaning |
|---|:--:|---|
| `SEALBOX_STORE_PATH` | ✓ | SQLite path, on the persistent volume |
| `SEALBOX_MASTER_KEY_PATH` | ✓ | Server master key(s) on the persistent volume, `0600`. Comma-separated, **most-current first** — see below. |
| `SEALBOX_LISTEN_ADDR` | ✓ | e.g. `0.0.0.0:8080` |
| `SEALBOX_PUBLIC_URL` | ✓ | Public HTTPS URL. **Also the WebAuthn relying-party ID** — changing it invalidates every registered passkey. |
| `SEALBOX_AUTH_TOKEN` | ✓ *(today)* | The single static bearer token. Replaced by identities and passkeys; see [ADR 0009](adr/0009-admin-authenticates-with-passkeys.md). |
| `SEALBOX_BOOTSTRAP_TOKEN` | first run *(planned)* | Accepted only while zero identities exist, within 30 minutes of start, once. Unset it afterwards. |

```toml
# fly.toml
[env]
  SEALBOX_STORE_PATH      = "/data/sealbox.db"
  SEALBOX_MASTER_KEY_PATH = "/data/master.pem"
  SEALBOX_LISTEN_ADDR     = "0.0.0.0:8080"
  SEALBOX_PUBLIC_URL      = "https://sealbox.example.dev"

[[mounts]]
  source      = "sealbox_data"
  destination = "/data"
```

```bash
fly secrets set SEALBOX_BOOTSTRAP_TOKEN=$(openssl rand -hex 32)
# after `sealbox init`:
fly secrets unset SEALBOX_BOOTSTRAP_TOKEN
```

### Master keys move as a list

`SEALBOX_MASTER_KEY_PATH` takes one or more paths. The first is the key new secrets are
encrypted under; any others are retired but still loaded, so the secrets already under them stay
readable and can be rekeyed onto the current one.

```bash
# steady state
SEALBOX_MASTER_KEY_PATH=/data/master.pem

# while rotating: the old key stays listed until nothing references it
SEALBOX_MASTER_KEY_PATH=/data/master-2.pem,/data/master-1.pem
```

Without this, rotating the server's master key would deadlock — the private half needed to read
the existing secrets would already be gone.

Sealbox will **not generate a key for you**. A mistyped path would silently produce a fresh key,
leaving every stored secret encrypted under one nobody holds, and the failure would only surface
later on a read. Generate it explicitly:

```bash
openssl genrsa -out master.pem 2048      # PKCS#8 and PKCS#1 are both accepted
chmod 600 master.pem
```

A secret whose master key is not in this list is **cold**: the server cannot decrypt it under any
circumstances, including rekey. That is the mechanism behind ADR 0001's two tiers, and it is not
an error state — it is how a credential is kept beyond the reach of a compromised server.

> **`SEALBOX_PUBLIC_URL` is load-bearing.** WebAuthn credentials are bound to a domain. Moving to a
> new hostname means re-registering every passkey — recoverable (the recovery key decrypts the
> database independently of authentication), but not something to discover during an incident.

### Durability

The server master key and the database are the only copies of everything. Neither is optional:

```toml
# litestream.yml
dbs:
  - path: /data/sealbox.db
    replicas:
      - type: s3
        bucket: sealbox-backups
        endpoint: https://s3.example.com
```

The master key is backed up **once, at initialisation**, as an encrypted recovery blob — not by
copying the file. See [`sealbox init`](cli-reference.md#sealbox-init) and
[ADR 0010](adr/0010-recovery-via-keypair-not-a-copied-key.md).

## Runner

The runner needs a server URL and, on first start only, a join token. Afterwards it authenticates
with a keypair it generated itself and stored locally.

| Variable | Meaning |
|---|---|
| `SEALBOX_SERVER` | Server URL |
| `SEALBOX_RUNNER_NAME` | Must match the `runner` field in the grants it should execute |
| `SEALBOX_JOIN_TOKEN` | First start only. Expires in 15 minutes. |
| `SEALBOX_STATE_PATH` | Where the runner keeps its own keypair |

```yaml
# runner-deployment.yaml (abridged)
spec:
  serviceAccountName: sealbox-runner        # its entire authority over the cluster
  containers:
    - name: runner
      image: ghcr.io/realmorrisliu/sealbox:latest
      args: ["runner"]
      env:
        - name: SEALBOX_SERVER
          value: https://sealbox.example.dev
        - name: SEALBOX_RUNNER_NAME
          value: prod-cluster
        - name: SEALBOX_JOIN_TOKEN
          valueFrom:
            secretKeyRef: { name: sealbox-join, key: token }
      volumeMounts:
        - { name: state, mountPath: /var/lib/sealbox }
```

**No Service, no Ingress, no inbound port.** The runner dials out.

`sealbox-join` is the one Secret you create by hand — and it stops being useful fifteen minutes
later, which is what makes that acceptable.

### Scoping the ServiceAccount

The runner's ServiceAccount is its **entire** authority over the cluster; every grant it executes
inherits it. Scope it to exactly what those grants need — for Secret synchronisation, permission
to create and update named Secrets and nothing else.

To narrow blast radius further, **run a second runner with a narrower ServiceAccount** and point
sensitive grants at it. The answer to finer permissions is another runner, not a policy engine
inside sealbox.

> **"Read-only" is not a safe scope.** A read-only kubeconfig can run
> `kubectl get secret -o yaml` and walk off with every Secret in the cluster. Any ServiceAccount
> reachable by an agent must exclude the `secrets` resource.

## CLI

`~/.config/sealbox/config.toml`:

```toml
server = "https://sealbox.example.dev"
identity = "claude-code"        # which identity this machine acts as
output = "table"                # table | json | yaml
```

Overridden by environment, then by flags:

| Variable | Overrides |
|---|---|
| `SEALBOX_SERVER` | `server` |
| `SEALBOX_TOKEN` | The bearer token, for agent and operator identities |
| `SEALBOX_OUTPUT` | `output` |

**There is no admin credential here.** Admin operations authenticate with a passkey per operation
or per `sealbox admin` session, and that session exists only in process memory
([ADR 0009](adr/0009-admin-authenticates-with-passkeys.md)). An agent that reads every file on
your machine finds nothing that can approve a grant.

Agent and runner identities do hold bearer tokens, since neither has a fingerprint. Their
authority is bounded: an agent may only run grants that already exist, and a runner may only claim
jobs addressed to it.

## Security notes

- **TLS is not optional.** Passkey authentication requires a secure context, and the server hands
  plaintext to runners. Fly.io terminates TLS for you.
- **CORS is not configured, because there is no cross-origin client.** The approval page is
  same-origin ([ADR 0004](adr/0004-no-web-ui.md)).
- **Rotate identity tokens by revoking and reissuing** — `sealbox identity revoke` takes effect
  immediately and affects nobody else.
