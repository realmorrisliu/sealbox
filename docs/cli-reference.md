# CLI Reference

> **Partly implemented.** `identity`, `audit`, `bootstrap`, and the secret commands exist and are
> documented below as they actually behave. `grant`, `run`, `rotate`, `runner`, and `admin`
> describe the target and do not exist yet. See [`README.md`](../README.md).

One binary, `sealbox`, used by humans, agents, and the runner alike. What differs is the identity
it authenticates as.

**There is no command that prints a secret's value.** That is not an omission.

## Command surface at a glance

Roles are ordered: each admits everything below it.

| Command | admin | operator | agent | runner |
|---|:--:|:--:|:--:|:--:|
| `audit`, read secrets and metadata | ✓ | ✓ | ✓ | |
| `run`, `rotate` (via an approved grant) *(target)* | ✓ | ✓ | ✓ | |
| store and delete secrets | ✓ | ✓ | | |
| `identity *`, master keys | ✓ | | | |
| `grant add/rm` *(target)* | ✓ | | | |
| `runner` *(target)* | | | | ✓ |

An identity that is authenticated but lacks the role gets **403**, distinct from the **401** of a
missing or unknown credential — so a caller can tell a missing credential from an insufficient one.

Admin commands will require a **passkey approval** rather than a stored token
([ADR 0009](adr/0009-admin-authenticates-with-passkeys.md)); today they use the admin identity's
token.

---

## Identities *(implemented)*

### `sealbox-cli bootstrap --token <value> [--name <name>]`

Claims a server that has no identities, creating the first admin and printing its token once.

```bash
sealbox-cli bootstrap --token "$SEALBOX_BOOTSTRAP_TOKEN" --name root
```

Refused unless all three hold: no identity exists, the token matches what the server was started
with, and the bootstrap window is still open. Unset the environment variable afterwards.

### `sealbox-cli identity create <name> --role <agent|operator|admin>`

Creates an identity and prints its token **once**. There is no way to retrieve it later; if it is
lost, revoke the identity and create another.

```bash
sealbox-cli identity create claude-code --role agent
sealbox-cli identity create alice --role operator
```

Roles are ordered — each admits everything below it:

| Role | May |
|---|---|
| `agent` | read secrets and metadata, read the audit trail, invoke approved capabilities |
| `operator` | additionally store and delete secrets |
| `admin` | additionally manage identities and master keys |

### `sealbox-cli identity list` / `identity revoke <name>`

Listing never shows tokens. Revocation takes effect on that identity's next request and affects
no one else; the identity is marked rather than deleted, so audit records naming it stay readable.

### `sealbox-cli audit [--identity X] [--action Y] [--since D] [--limit N]`

```bash
sealbox-cli audit --since 24h
sealbox-cli audit --identity claude-code --since 7d
sealbox-cli audit --action "PUT /v1/secrets/db-password"
```

`--since` takes `90s`, `30m`, `24h`, `7d`, or a Unix timestamp.

Every attempt is recorded, including refusals — an agent reaching for something its role does not
permit is exactly the signal worth having. Readable by every identity, including agents:
concealing it protects nothing an agent could not already observe.

Records name the *resource*, never its value.

---

## Secrets *(implemented)*

### `sealbox-cli secret set <key> [--ttl N]`

Reads the value from **stdin**. There is no argument form — while one exists it gets used, and
every use puts a credential into shell history and into `ps` output for every user on the machine.

```bash
printf %s "$VALUE" | sealbox-cli secret set app/database-url
sealbox-cli secret set app/token            # prompts, hidden, when run from a terminal
sealbox-cli secret set k8s/dockerconfig < config.json
```

Only a trailing newline is stripped — the artefact of a pipe. Leading and interior whitespace
survive, because silently altering a credential is worse than storing an odd one.

### `sealbox-cli secret gen <key> [--type password|hex] [--length N] [--ttl N]`

The server generates the value, encrypts it, and stores it. **The plaintext never crosses the
network and is not returned to anyone — including the caller who asked for it.** That is what
lets an agent provision a credential it can never read.

```bash
sealbox-cli secret gen app/session-key                       # password, 32 characters
sealbox-cli secret gen app/hmac --type hex --length 32       # 32 bytes, 64 hex characters
```

`password` is alphanumeric without `0`/`O` or `1`/`l`/`I`, and without punctuation. Symbols in a
generated credential cost more in shell quoting, connection-string escaping, and YAML type
guessing than they add in entropy; length is the cheaper way to buy it. 32 characters of this
alphabet is about 187 bits.

A length below 16 is refused rather than honoured — a caller asking for eight is likelier to have
made a mistake than to have a reason, and a weak credential looks exactly like a strong one.

### `sealbox-cli secret list`

Keys, versions, and timestamps. Never values. Expired secrets are omitted.

### `sealbox-cli secret get <key> [--version N]` *(being replaced)*

Returns ciphertext for the client to decrypt with its own private key. The target design has no
command that yields a value at all — a secret is used through a grant, on a runner. This remains
only because nothing else can consume a secret until runners exist.

---

## Setup *(target)*

### `sealbox init`

One-time initialisation of a fresh server: generates the recovery keypair locally, has the server
generate its master key, forces recovery-key verification, and registers the first passkey.

```bash
sealbox init --server https://sealbox.example.dev --bootstrap-token <value>
```

| Flag | Meaning |
|---|---|
| `--server <url>` | Server URL; stored in the local config |
| `--bootstrap-token <value>` | The value injected at deploy time. Single use, 30-minute window, zero-identity only. |

### `sealbox admin`

Opens an interactive session after **one** passkey authentication. The session lives in process
memory and is never written to disk; it dies with the process.

```bash
sealbox admin
> set app/database-url
> grant add ./grants/k8s-sync.toml
> exit

sealbox admin --exec 'set app/token'      # non-interactive, one authentication
```

Without this, importing fifty credentials would mean fifty fingerprints — and intolerable security
gets bypassed.

---

## Secrets

### `sealbox set <name>`

Stores a value read from **stdin** — never from a command-line argument, which would put it in
shell history and process listings.

```bash
sealbox set app/database-url                 # prompts, input hidden
sealbox set k8s/dockerconfig < config.json
```

### `sealbox gen <name>`

Generates a value **on the server** and stores it. The plaintext never crosses the network.

```bash
sealbox gen app/session-key --type password --length 32
sealbox gen app/hmac --type hex
```

| Flag | Values |
|---|---|
| `--type` | `password`, `hex` |
| `--length <n>` | Length in characters or bytes depending on type |
| `--ttl <seconds>` | Optional expiry |

### `sealbox ls [prefix]`

Lists names and metadata. **Never values.**

```bash
sealbox ls app/
sealbox ls --uses pg/prod-admin-password     # which grants may use this secret
```

`--uses` answers the question no other secret manager can: *everything this credential can do
here*. The answer is the set of grants declaring it.

### `sealbox rm <name> [--version N]`

Deletes a secret, or one version of it.

---

## Grants

### `sealbox grant add <file>`

Submits a grant for approval. Opens a browser; the server renders what is being approved, and a
passkey signs it.

```bash
sealbox grant add ./grants/k8s-sync.toml
```

The script body, if any, is **ingested and stored**, never referenced by path — otherwise a grant
approved once could have its file edited afterwards, and what was reviewed would differ from what
runs.

### `sealbox grants` / `sealbox grant show <name>` / `sealbox grant rm <name>`

Lists what you may run, shows one grant's declaration, removes one. `rm` is an admin operation.

### Grant file format

```toml
[k8s-sync]
adapter = "kubernetes-secret"          # or: script = """ ... """
runner  = "prod-cluster"
config  = { namespace = "production", name = "app-runtime-secrets" }
secrets = { DATABASE_URL = "app/database-url", OSS_ENDPOINT = "app/oss-endpoint" }
then    = ["k8s-restart", "verify-health"]     # optional linear chain, stop on failure
```

| Field | Meaning |
|---|---|
| `adapter` | A built-in implementation. Mutually exclusive with `script`. |
| `script` | Shell body, for anything adapters do not cover. Stored, not referenced. |
| `runner` | Which runner executes this. |
| `config` | Adapter-specific settings. |
| `secrets` | Name-to-secret mapping. **This is what a human reviews.** |
| `then` | Grants to run on success, in order. No retries, no branching ([ADR 0011](adr/0011-rotation-uses-dual-credentials-and-a-linear-chain.md)). |

Parameters written `{name}` are substituted from `run` arguments **into argv, never through a
shell** — so `{ns}` = `x; curl evil.com` is merely an odd argument.

Built-in adapters: `kubernetes-secret`, `postgres-role`.

---

## Execution

### `sealbox run <grant> [key=value ...]`

Submits a job, waits, prints the result.

```bash
sealbox run k8s-sync ns=production
sealbox run k8s-restart ns=production deploy=api
```

The CLI receives an exit code and output — **never plaintext**. Execution happens on the runner
the grant declares.

### `sealbox rotate <secret> --via <grant> [--from-output] [key=value ...]`

Replaces a secret's value, committing **only if the grant succeeds**. A failed upstream push
leaves the old value current.

```bash
sealbox rotate app/db-password --via pg-set-password
sealbox rotate app/database-url --via pg-provision --from-output host=pgm-x user=app db=app
```

| | Value handed to the grant | Value stored |
|---|---|---|
| default | server-generated, as `$SEALBOX_NEW` | the generated value |
| `--from-output` | server-generated, as `$SEALBOX_NEW` | the grant's stdout |

`--from-output` is how composed values are handled — a `DATABASE_URL` with a percent-encoded
password, or a credential only an upstream can issue. The two switches are orthogonal: sealbox
always generates, the grant decides what is worth storing.

### `sealbox runner --name <name>`

Runs the executor. Long-polls for jobs, receives plaintext for the grants it is given, executes,
reports back. Outbound connections only.

```bash
sealbox runner --name prod-cluster        # a Deployment in your cluster
sealbox runner --name laptop              # for grants targeting your own machine
```

Running something locally means running a local runner. There is exactly one execution path, so
there is exactly one security model.

---

## Identities

### `sealbox identity create <name> --role <role>`

```bash
sealbox identity create alice --role operator      # → single-use invite, 24h, bound to alice
sealbox identity create prod-cluster --role runner # → join token, 15 min, single use
sealbox identity create claude-code --role agent   # → bearer token
```

Humans get an **invite link** and register their own passkey; the link grants only the right to
register, carries no data access, and is fully audited. Runners get a **join token** they exchange
for a self-generated keypair. Agents get a bearer token, because they have no fingers.

### `sealbox identity list` / `sealbox identity revoke <name>`

Revocation is immediate and affects nobody else. That is what identities are for.

---

## Audit and recovery

### `sealbox audit`

```bash
sealbox audit --since 24h
sealbox audit --identity claude-code
sealbox audit --grant k8s-sync
```

Every attempt is recorded, successful or not: who, when, which grant, which secrets, what outcome.

### `sealbox recovery-export` / `sealbox recovery-restore`

Exports the encrypted recovery blob — safe to store anywhere, because it is encrypted to a key the
server does not hold. `recovery-restore` rebuilds a server's master key from it given the recovery
key.

---

## Configuration

Server URL and identity token live in `~/.config/sealbox/config.toml`; see
[Configuration](configuration.md). Admin operations do not read a credential from that file,
because none exists.
