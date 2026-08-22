# Configuration

> **Mostly implemented.** Everything in the server section works today, `SEALBOX_PUBLIC_URL`
> included — it is required, and startup fails without it, because every passkey is bound to it.
> Identities, roles, the audit trail, and passkeys all exist. **Join tokens do not:** a runner's
> token is a long-lived identity token rather than one exchanged at first contact. See
> [`README.md`](../README.md).

Three things are configured separately, because they are trusted differently: the **server** holds
everything, the **runner** holds plaintext transiently, and the **CLI** holds nothing but a
pointer and an identity.

## Server

Environment variables. On Fly.io, non-secret values go in `fly.toml`, secrets via `fly secrets`.

| Variable | Required | Meaning |
|---|:--:|---|
| `SEALBOX_STORE_PATH` | ✓ | SQLite path, on the persistent volume |
| `SEALBOX_MASTER_KEY_PATH` | ✓ | Server master key(s) on the persistent volume, `0600`. Comma-separated, **most-current first** — see below. Generated on first boot if a single path is configured and the store is empty; missing in any other case is fatal. |
| `SEALBOX_LISTEN_ADDR` | ✓ | e.g. `0.0.0.0:8080` |
| `SEALBOX_PUBLIC_URL` | ✓ | Public HTTPS URL. **Also the WebAuthn relying-party ID** — changing it invalidates every registered passkey. |
| `SEALBOX_BOOTSTRAP_TOKEN` | first run | Creates the first admin. Accepted only while zero identities exist and only within the window below. Unset it afterwards. |
| `SEALBOX_BOOTSTRAP_WINDOW_SECS` | | How long the bootstrap token stays usable. Default 1800. |
| `SEALBOX_REPLICATION_METRICS_URL` | | Where Litestream serves metrics, e.g. `http://127.0.0.1:9090/metrics`. Unset means replication is not watched — reported as *not configured*, which is not the same as fine. |

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
# after claiming the server:
fly secrets unset SEALBOX_BOOTSTRAP_TOKEN
```

The window exists because leaving the token in the environment after use is the normal outcome,
not the exceptional one. After it closes, the token is refused even on a server with no
identities — restart to reopen it.

### The first key is generated, the rest are yours

On a server that has never held anything — no master key and no secret in the store, exactly one
path configured, no file at it — the server generates the key itself, `0600`, and logs a
fingerprint rather than the key. Hosted platforms are why: a volume exists only while a machine is
attached, so there is no moment before the first boot in which to place a file.

Every other case is still fatal. In particular, **a restored database with no key file refuses to
start** rather than generating one: that is a volume lost and replication restored, and a fresh key
there would replace the only thing that can read what came back.

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

### Replication is watched, not assumed

Litestream fails silently: a replica that stopped three weeks ago looks exactly like a working one
until someone restores from it. So the server asks it, once a minute, whether it is still going —
`litestream_sync_count` must move, because Litestream syncs on a one-second ticker whether or not
anything was written, and `litestream_sync_error_count` must not.

```yaml
# litestream.yml
addr: ":9090"          # without this there are no metrics to ask for
dbs:
  - path: /data/sealbox.db
    replicas: [...]
```

`GET /healthz/replication` answers **503** while it is failing, and each transition is written to
the audit trail once — edge-triggered, so a week-long outage is two records rather than ten
thousand.

It deliberately does **not** affect `/healthz/ready`. A server that stops serving because its
backup is stale has turned a recoverable problem into an outage, and taking the machine out of
rotation does not fix replication.

Litestream covers the database and **not** the master key. A replicated database without its key
is ciphertext under a key that no longer exists, which looks like a healthy backup right up until
the restore.

The backup is an encrypted recovery blob, not a copy of the file: `recovery init` generates a
keypair locally, uploads only the public half, and verifies the result by recovering the master key
with it ([ADR 0010](adr/0010-recovery-via-keypair-not-a-copied-key.md)). The blob is re-made
whenever the master key changes, and `recovery restore` needs no server — which matters, because
the server is what you have lost.

## Runner

> **Implemented today** with an ordinary identity token; the join-token exchange below is the
> target.

A runner is an identity with the `runner` role, whose name matches the `runner` field of the
grants it executes:

```bash
sealbox-cli identity create prod-cluster --role runner   # prints its token once
sealbox-cli runner --name prod-cluster                   # with that token configured
```

Its permissions are **disjoint** from every other role: it may claim jobs addressed to it and
report their results, and nothing else — it cannot invoke a grant, read a secret by name, list
secrets, or read the audit trail. Nor can any other role claim a job: an admin is refused there
too, because being the most privileged identity does not make you the machine a job was addressed
to.

| Variable | Meaning |
|---|---|
| `SEALBOX_SERVER` | Server URL |
| `SEALBOX_TOKEN` | The runner identity's token |

The runner authenticates with **workload identity**: `--token-file` points at a projected
ServiceAccount token, re-read before every poll because the platform rotates it. `SEALBOX_TOKEN` is
then unnecessary — there is no sealbox credential in the cluster at all. See
[Getting started](getting-started.md#4-put-a-runner-in-your-cluster-implemented).

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

**The image must carry `kubectl` and `psql`** if grants use the built-in adapters — they shell
out rather than linking client libraries, and `kubectl` picks up the ServiceAccount on its own
inside a cluster. A missing tool fails the job with a message saying so.

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

`token` is **this machine's identity token**, not a shared secret — every caller has its own, and
revoking one affects nobody else. It is shown once, when the identity is created.

Roles are ordered, and each admits everything below it:

| Role | May |
|---|---|
| `agent` | read secrets and metadata, read the audit trail, invoke approved capabilities |
| `operator` | additionally store and delete secrets |
| `admin` | additionally manage identities and master keys |

> **Not yet: passkeys.** [ADR 0009](adr/0009-admin-authenticates-with-passkeys.md) replaces admin
> authentication so that no admin credential exists on disk at all. Until that lands, an admin
> token is a file on your machine — keep it off machines where an agent runs, or accept that an
> agent there could read it.

## Security notes

- **TLS is not optional.** Passkey authentication requires a secure context, and the server hands
  plaintext to runners. Fly.io terminates TLS for you.
- **CORS is not configured, because there is no cross-origin client.** The approval page is
  same-origin ([ADR 0004](adr/0004-no-web-ui.md)).
- **Rotate identity tokens by revoking and reissuing** — `sealbox identity revoke` takes effect
  immediately and affects nobody else.
