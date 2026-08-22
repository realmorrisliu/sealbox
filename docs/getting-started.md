# Getting Started

> **Mostly implemented.** Bootstrap, identities, passkeys, secrets, grants, and the runner all
> work. The recovery-keypair ceremony in step 2 does not exist yet. Each step is marked. See
> [`README.md`](../README.md).

Setting sealbox up takes about half an hour, once. Every step exists for a reason; none of them is
boilerplate.

## 1. Bring the server up *(implemented)*

```bash
fly launch --no-deploy            # fly.toml is in the repo; keep it
fly volumes create sealbox_data --size 1

# You generate the bootstrap token. It must never pass through logs.
fly secrets set SEALBOX_BOOTSTRAP_TOKEN=$(openssl rand -hex 32)
fly deploy
```

Set `SEALBOX_PUBLIC_URL` in `fly.toml` to the hostname people will actually reach **before the
first deploy**. It is the WebAuthn relying-party ID: changing it later invalidates every registered
passkey.

The bootstrap token is accepted **only while zero identities exist**, **only within 30 minutes of
server start**, and **exactly once**; the use is audited. Unset it immediately afterwards:

```bash
fly secrets unset SEALBOX_BOOTSTRAP_TOKEN
```

> **Why not print it in the logs?** Logs get shipped, aggregated, retained, and read by people who
> should not be able to claim your server. This is the same reason GitLab and Grafana take their
> initial credential from the environment.

### Back up the master key. Now, not later.

On its first start the server finds no key and nothing stored, so it generates one at
`/data/master.pem` and logs a fingerprint — never the key itself. **That file is the only copy**,
and Litestream replicates the database, not the key: losing the volume without a backup means the
replicated database is ciphertext under a key that no longer exists.

```bash
sealbox-cli admin recovery init --out ./sealbox-recovery.pem --description "your name here"
```

The CLI generates a recovery keypair **locally**, sends only the public half, and the server stores
its master key encrypted under it. Then it proves the result works: it fetches what the server
stored and recovers the master key with the file it just wrote, refusing to report success
otherwise. An unverified backup is reliably not a backup.

Move `sealbox-recovery.pem` into a password manager or onto paper and delete it from the machine —
an agent on your laptop can read it exactly as easily as you can. Keep a copy of the blob too if
you like; it is safe anywhere, because the server does not hold the key that opens it:

```bash
sealbox-cli admin recovery export <id> --out ./sealbox-blob.json
```

The blob is re-made automatically whenever the master key changes, so it cannot quietly stop
matching what it is meant to restore.

**Recovering** needs no server, which is the point — the server is what you have lost:

```bash
sealbox-cli recovery restore --blob ./sealbox-blob.json --key ./sealbox-recovery.pem --out master.pem
litestream restore -o sealbox.db s3://sealbox-backups/sealbox
```

> **Two people can each hold one.** Registering a second recovery key does not retire the first;
> both get their own blob, and neither has to be shared.

### Replication

Litestream supervises the server, so replication and the server start and stop together. Point it
at object storage in `/data/litestream.yml`:

```yaml
dbs:
  - path: /data/sealbox.db
    replicas:
      - type: s3
        bucket: sealbox-backups
        path: sealbox
```

With no configuration file it runs the server and replicates nothing, which is the right behaviour
for a local run and the wrong one for production — check `fly logs` for Litestream announcing the
replica after the first deploy.

## 2. Become admin *(partly implemented)*

```bash
sealbox-cli bootstrap --token <value> --name root
```

This prints **no token**. An admin has no credential to store — that is the point (ADR 0009) — so
what comes back is an enrolment link:

```
Open this to register your passkey:
  https://sealbox.example.dev/enrol/6f2c…
```

Open it, register the authenticator you actually carry, and unset the bootstrap token. From then
on, every admin operation goes through `sealbox-cli admin <command>`, which opens a session that
lives in that process's memory and dies with it. A bearer token is refused on admin routes even if
it belongs to a valid admin identity, so there is nothing on your machine an agent could read and
use to act as you.

The link is single use, expires in thirty minutes, and works only while that identity has no
authenticator: a leaked link must be a way to become *an* admin for the first time, never a way to
displace a working one.

> **Not built yet:** the recovery-keypair ceremony — generating a recovery keypair locally, storing
> the master key encrypted under its public half, and forcing you to type the private half back
> before initialisation completes. Until it exists, the server's master key file *is* the backup;
> keep a copy of it somewhere you keep nothing else.

> **The re-entry is not ceremony for its own sake.** An unverified backup is reliably not a backup
> — it is a transcription error nobody discovers until the day it matters. This is why 1Password
> makes you print an Emergency Kit and hardware wallets make you re-enter the seed phrase.

Store the recovery key where you store nothing else — a password manager, or paper. It is what
stands between a lost server and lost credentials. Since it is also a master key the server does
not hold ([ADR 0001](adr/0001-broker-over-e2ee.md)), it decrypts the database directly, and works
even if every passkey is lost. **Authentication and encryption fail independently, by design.**

## 3. Create an identity for each caller *(implemented)*

```bash
sealbox-cli identity create claude-code --role agent
sealbox-cli identity create alice --role operator
```

Each token is printed once — except an admin's, which does not exist: `--role admin` returns an
enrolment link, as in step 2. Give an agent the narrowest role that lets it do its job: an `agent`
reads and invokes but cannot store secrets or manage identities, and every attempt it makes is
recorded whether it succeeds or is refused.

Revoking one identity is immediate and affects no one else — which is the whole reason there is
no shared token.

## 4. Put a runner in your cluster *(implemented)*

The runner holds **no sealbox credential**. It presents the ServiceAccount token its own cluster
signs, and sealbox verifies that offline against keys you register once.

```bash
# once per cluster
kubectl get --raw /openid/v1/jwks > jwks.json
sealbox-cli admin issuer add prod-cluster \
  --issuer-url "$(kubectl get --raw /.well-known/openid-configuration | jq -r .issuer)" \
  --jwks-file jwks.json

# one identity, one exact ServiceAccount
sealbox-cli admin identity create prod-runner --role runner \
  --issuer prod-cluster \
  --subject system:serviceaccount:sealbox:runner \
  --audience sealbox
```

In the Deployment, mount a **projected** token with that audience — not the default one, or a
token minted for the cluster's API server would authenticate here — and point the runner at it:

```yaml
      containers:
        - name: runner
          args: ["runner", "--name", "prod-runner", "--token-file", "/var/run/sealbox/token"]
          env:
            - name: SEALBOX_SERVER
              value: https://sealbox.example.dev
          volumeMounts:
            - name: sealbox-identity
              mountPath: /var/run/sealbox
              readOnly: true
      volumes:
        - name: sealbox-identity
          projected:
            sources:
              - serviceAccountToken:
                  path: token
                  audience: sealbox
                  expirationSeconds: 3600
```

The runner re-reads that file before every poll, because the kubelet rotates it underneath.

This is the only place a grant executes and the only place a secret's plaintext exists outside
the server. It dials out, so the cluster needs no inbound port and no public endpoint.

Its permissions are disjoint from every other role: claim, execute, report — nothing else. An
admin cannot claim a job either, because the most privileged identity is still not the machine
the job was addressed to.

> **Subjects match exactly.** A prefix would mean that creating a ServiceAccount is enough to
> become a runner, and in most clusters far more people can create one than can be trusted with
> plaintext. Revocation stays sealbox's: revoking the identity ends it regardless of what the
> cluster keeps signing.

> **A cluster that rotates its signing keys** needs its JWKS re-registered. Register the document
> holding both keys with `issuer update`, and remove the old one once nothing presents it — the
> same overlap master keys use.

The runner needs **no inbound port, no Ingress, and no public endpoint** — it dials out. Its
ServiceAccount is its entire authority over the cluster; scope it to exactly what your grants
need, and add a second runner with a narrower ServiceAccount rather than reaching for a
permissions system inside sealbox.

## 5. Move your credentials in *(implemented)*

```bash
printf %s "$DATABASE_URL" | sealbox-cli secret set app/database-url
printf %s "$OSS_ENDPOINT" | sealbox-cli secret set app/oss-endpoint
```

The value comes from stdin; there is no argument form, so nothing lands in shell history or in
`ps` output.

Anything that is just a random number should be generated instead of carried in — then it has
never existed anywhere else:

```bash
sealbox-cli secret gen app/session-key
sealbox-cli secret gen app/hmac --type hex
```

Storing secrets is an `operator` operation, not an admin one, so bulk work is an ordinary shell
loop and no fingerprint is involved:

```bash
for f in ~/creds/*; do sealbox-cli secret set "app/$(basename "$f")" < "$f"; done
```

Then delete the originals:

```bash
rm ~/.config/app/secrets.env
```

## 6. Grant the first capability *(implemented)*

Have an agent draft it by imitating [`examples/grants/`](../examples/grants/):

```toml
[k8s-sync]
adapter = "kubernetes-secret"
runner  = "prod-cluster"
config  = { namespace = "production", name = "app-runtime-secrets" }
secrets = { DATABASE_URL = "app/database-url", OSS_ENDPOINT = "app/oss-endpoint" }
```

```bash
sealbox-cli grant add ./grants/k8s-sync.toml
sealbox-cli run k8s-sync
```

Submitting creates nothing. The command prints an approval URL and opens it; the page lists the
secrets first, and you sign it with your passkey — **on that machine or on your phone**. Sealbox
confines the implementation to exactly those secrets, so that line is what the approval is about.

That the page comes from the server is the whole point. Terminal output is written by whatever
process is running, so an agent could show one grant and submit another; a page rendered from what
the server stored cannot be influenced that way, and the signature is bound to it.

## From then on

```bash
# Agents, daily
sealbox-cli run k8s-sync ns=production
sealbox-cli rotate app/database-url --via pg-provision --from-output host=... user=app

# You, occasionally — the first is a browser prompt, the last needs your passkey
sealbox-cli grant add ./grants/new-thing.toml
sealbox-cli audit --since 24h
sealbox-cli admin identity revoke agent-laptop
```

## Recovering from total loss

```bash
fly launch                    # new instance, same domain
sealbox recovery-restore      # supply the recovery key; decrypts the master key
litestream restore            # bring back the database
```

Passkeys still work because WebAuthn binds to the domain, and the runner reconnects on its own.

## Next

- [Agent skill](../skills/sealbox/SKILL.md) — copy it into your agent's skills directory
- [CLI reference](cli-reference.md) — the full command surface
- [Configuration](configuration.md) — server, runner, and CLI settings
- [Design](agent-native-design.md) — why any of this is shaped the way it is
