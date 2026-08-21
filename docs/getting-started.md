# Getting Started

> **Partly implemented.** Steps 1, 2, and 3 work today in a simpler form — bootstrap and identity
> creation exist; the recovery-keypair ceremony and passkeys do not. The runner and grants are the
> target. Each step is marked. See [`README.md`](../README.md).

Setting sealbox up takes about half an hour, once. Every step exists for a reason; none of them is
boilerplate.

## 1. Bring the server up *(implemented)*

```bash
fly launch
fly volumes create sealbox_data --size 1

# You generate the bootstrap token. It must never pass through logs.
fly secrets set SEALBOX_BOOTSTRAP_TOKEN=$(openssl rand -hex 32)
fly deploy
```

The bootstrap token is accepted **only while zero identities exist**, **only within 30 minutes of
server start**, and **exactly once**; the use is audited. Unset it immediately afterwards:

```bash
fly secrets unset SEALBOX_BOOTSTRAP_TOKEN
```

> **Why not print it in the logs?** Logs get shipped, aggregated, retained, and read by people who
> should not be able to claim your server. This is the same reason GitLab and Grafana take their
> initial credential from the environment.

## 2. Become admin *(partly implemented)*

> **Implemented today**, in a simpler form: `sealbox-cli bootstrap --token <value> --name root`
> creates the first admin and prints its token once. The recovery keypair ceremony and passkey
> registration described below are the target and do not exist yet.

```bash
sealbox init --server https://sealbox.example.dev --bootstrap-token <value>
```

This runs one ceremony with several parts:

1. The CLI generates a **recovery keypair locally**. The public half is uploaded; the private half
   is displayed **once**.
2. The server generates its master key and stores it encrypted under your recovery public key.
   **The master key itself is never displayed, logged, or returned by any endpoint.**
3. You must **type the recovery key back** before initialisation completes.
4. A browser opens; you register your passkey.

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

Each token is printed once. Give an agent the narrowest role that lets it do its job: an `agent`
reads and invokes but cannot store secrets or manage identities, and every attempt it makes is
recorded whether it succeeds or is refused.

Revoking one identity is immediate and affects no one else — which is the whole reason there is
no shared token.

## 4. Put a runner in your cluster *(target)*

```bash
sealbox identity create prod-cluster --role runner   # → join token, 15 minutes, single use

kubectl -n sealbox create secret generic sealbox-join \
  --from-literal=token=<join-token>
kubectl apply -f runner-deployment.yaml
```

On first start the runner generates **its own keypair**, registers the public half using the join
token, and authenticates by signature thereafter. The join token expires in fifteen minutes, so
**the Secret you just created becomes worthless within the hour** — which is what makes this one
manual step acceptable.

The chicken-and-egg has to be broken somewhere. It is broken here, deliberately, visibly, and with
a short fuse.

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

> **The target**, once passkeys land, is one authentication for a whole session:
>
> ```bash
> sealbox admin
> > set app/database-url
> > exit                               # the session lives in memory and dies with the process
> ```

Then delete the originals:

```bash
rm ~/.config/app/secrets.env
```

No import command is needed for bulk work — a shell loop inside one admin session does it:

```bash
sealbox admin --exec 'for f in ~/creds/*; do set app/$(basename $f) < $f; done'
```

## 6. Grant the first capability *(target)*

Have an agent draft it by imitating [`examples/grants/`](../examples/grants/):

```toml
[k8s-sync]
adapter = "kubernetes-secret"
runner  = "prod-cluster"
config  = { namespace = "production", name = "app-runtime-secrets" }
secrets = { DATABASE_URL = "app/database-url", OSS_ENDPOINT = "app/oss-endpoint" }
```

```bash
sealbox grant add ./grants/k8s-sync.toml
```

A browser opens — on your laptop, or scan the link with your phone. **What you approve is the
declaration above, not a script.** The page is rendered by the server, so an agent cannot show you
one grant and submit another.

Approving from a phone is worth doing: it puts the approval on a device the agent has no access
to at all.

## From then on

```bash
# Agents, daily
sealbox run k8s-sync ns=production
sealbox rotate app/database-url --via pg-provision --from-output host=... user=app

# You, occasionally
sealbox grant add ./grants/new-thing.toml
sealbox audit --since 24h
sealbox identity revoke agent-laptop
```

## Recovering from total loss

```bash
fly launch                    # new instance, same domain
sealbox recovery-restore      # supply the recovery key; decrypts the master key
litestream restore            # bring back the database
```

Passkeys still work because WebAuthn binds to the domain, and the runner reconnects on its own.

## Next

- [CLI reference](cli-reference.md) — the full command surface
- [Configuration](configuration.md) — server, runner, and CLI settings
- [Design](agent-native-design.md) — why any of this is shaped the way it is
