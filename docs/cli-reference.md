# CLI Reference

> **Mostly implemented.** `identity`, `audit`, `bootstrap`, the secret commands, `grant`, `run`,
> `runner`, `rotate`, and `admin` all exist and are documented as they actually behave. What
> cannot be exercised without a person is the browser half of each passkey ceremony. See
> [`README.md`](../README.md).

One binary, `sealbox-cli`, used by humans, agents, and the runner alike. What differs is the
identity it authenticates as.

Giving this file to an agent is the wrong move — it is reference material, and an agent needs the
habits instead. Use [`skills/sealbox/SKILL.md`](../skills/sealbox/SKILL.md), which links here for
detail.

**There is no command that prints a secret's value.** That is not an omission.

## Command surface at a glance

Roles are ordered: each admits everything below it.

| Command | admin | operator | agent | runner |
|---|:--:|:--:|:--:|:--:|
| `audit`, read secrets and metadata | ✓ | ✓ | ✓ | |
| `run`, `rotate` (via an approved grant) | ✓ | ✓ | ✓ | |
| `grant add` — submit a draft for approval | ✓ | ✓ | ✓ | |
| store and delete secrets | ✓ | ✓ | | |
| `identity *`, master keys, `grant rm` | passkey | | | |
| `runner` | | | | ✓ |

An identity that is authenticated but lacks the role gets **403**, distinct from the **401** of a
missing or unknown credential — so a caller can tell a missing credential from an insufficient one.

Admin operations are the exception to the table's logic: they need a **passkey session**, and a
bearer token is refused outright — even a valid admin identity's
([ADR 0009](adr/0009-admin-authenticates-with-passkeys.md)). A route that still accepted one would
leave the hole open for every caller that simply forgot to stop sending it.

```bash
sealbox-cli admin identity create alice --role operator
sealbox-cli admin identity revoke agent-laptop
sealbox-cli admin grant rm k8s-sync
```

It prints a sign-in URL and opens it. Sign in there — **on this machine or on your phone** — and
the session travels back to the waiting process. It lives in that process's memory, expires on its
own, and is never printed or written down: a credential a human copies is a credential that ends
up in scrollback.

Admin operations are rare by design. The frequent work — storing secrets, running grants — belongs
to `operator` and `agent`, which authenticate with tokens, so nothing routine is behind a
fingerprint and bulk import is an ordinary shell loop.

Submitting a grant is deliberately *not* an admin operation: it creates nothing, so an agent may
draft one. The grant exists only once a human signs for it on the page the server renders.

---

## Identities *(implemented)*

### `sealbox-cli bootstrap --token <value> [--name <name>]`

Claims a server that has no identities, creating the first admin. It prints **no token** — an
admin has no credential to store (ADR 0009) — but an enrolment link to open in a browser, where
you register the passkey that authenticates you from then on.

```bash
sealbox-cli bootstrap --token "$SEALBOX_BOOTSTRAP_TOKEN" --name root
```

Refused unless all three hold: no identity exists, the token matches what the server was started
with, and the bootstrap window is still open. Unset the environment variable afterwards.

### `sealbox-cli identity create <name> --role <agent|operator|admin|runner>`

Creates an identity and prints its token **once**. There is no way to retrieve it later; if it is
lost, revoke the identity and create another.

An `admin` is the exception: it gets an enrolment link instead of a token, valid once, for thirty
minutes, and only while that identity has no authenticator yet. A leaked link must be a way to
become *an* admin for the first time, never a way to displace a working one.

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

## Grants *(implemented)*

A grant is a permitted use of secrets: which ones, what is done with them, and where it runs.

### `sealbox-cli grant add <file>`

Submits a grant from a TOML file for approval. Any identity may draft one — nothing is created
until a human signs for it. The file is parsed locally, so a malformed one fails with the file in
hand.

```toml
[k8s-sync]
adapter = "kubernetes-secret"
runner  = "prod-cluster"
config  = { namespace = "utopia-system", name = "utopia-runtime-secret-bridge" }
secrets = { DATABASE_URL = "utopia/prod/database-url" }
```

| Field | Meaning |
|---|---|
| `adapter` + `config` | A built-in implementation. Mutually exclusive with `script`. |
| `script` + `command` | The escape hatch. The body is **stored**, never referenced by path. `command` is the argv. |
| `runner` | Which runner executes it. |
| `secrets` | Injected name → secret name. **This is what you are approving.** |
| `then` | Grants to run on success, in order. Linear, stop-on-failure. |

The command prints the declaration and then an approval URL, and opens it. **What you sign is the
page, not the terminal output.** A terminal's output is written by whatever process is running, so
an agent could print one grant and submit another; the page is rendered by the server from what the
server stored, and the signature is bound to it.

The page shows the secrets first — sealbox confines the implementation to exactly those, so that
line is the security-relevant part. A script's body is deliberately not shown: judging one is a
hard cognitive task, and that kind of review decays into a glance; judging one line does not.

Approve on your phone if you like. The command waits, and picks it up when it lands. If nobody
approves within three minutes it gives up, having created nothing.

**Secret names are literal, never parameterised.** `"app/{env}/url"` is refused: the parameter
would come from whoever invokes the grant, letting them choose which credential it reaches. Two
environments are two grants — which also makes approving production its own deliberate act.

Everything checkable is checked now, while you are here to fix it: the secrets exist, the
adapter is known, the chain resolves and does not cycle.

### `sealbox-cli grant list` / `grant show <name>` / `grant rm <name>`

Any identity may list, show, and submit a draft. Removing one is an admin operation, behind a
passkey — `sealbox-cli admin grant rm <name>`.

**There is no update.** Changing a grant means removing it and adding the replacement, which puts
the new declaration in front of a human exactly as the first one was. An update endpoint would be
the natural place for a capability to widen quietly: a `secrets` list gaining one entry is a
one-line diff that reads like nothing.

### `sealbox-cli secret uses <key>`

```console
$ sealbox-cli secret uses pg/prod-admin-password
pg-provision
rotate-utopia-db
```

**Two lines: everything that credential can do in this system.** In Vault, 1Password, or GitHub
Secrets the answer is "anything, to anyone holding it".

---

## Running *(implemented)*

### `sealbox-cli run <grant> [key=value ...]`

Submits a job, waits, and prints the exit status and whatever the implementation printed.
**Never a secret value.**

```bash
sealbox-cli run k8s-sync
sealbox-cli run k8s-restart deploy=api
```

A caller supplies a grant name and parameters — never a command. Parameters are substituted into
the implementation's argv as whole tokens and are never re-parsed, so a parameter of
`x; curl evil.com` arrives as one odd argument and nothing in it executes.

### `sealbox-cli rotate <secret> --via <grant> [--from-output] [key=value ...]`

Replaces a secret's value, committing **only if the grant succeeds**.

```bash
sealbox-cli rotate app/db-password --via pg-set-password
sealbox-cli rotate app/db-url --via pg-provision --from-output
```

The server generates the new value and hands it to the grant as `$SEALBOX_NEW`, injected exactly
like a declared secret — **an implementation never produces secret material**, so randomness stays
in one audited place instead of being reimplemented per script. A caller supplying a value is
refused, not honoured.

**If the grant fails, the previous value is still current, unchanged.** That is the whole point:
a stored credential that silently disagrees with reality is worse than no rotation, because
nothing says so.

`--from-output` stores what the grant printed instead, for values that are *composed* — a
`DATABASE_URL` with a percent-encoded password — or issued upstream. A capturing implementation
**prints the value on stdout and nothing else**; diagnostics go to stderr. Printing nothing fails
the rotation: storing an empty credential because a script forgot is the same failure as storing
the wrong one.

The captured value never enters the job record. Job output is stored in the clear for you to read;
a credential must not travel that way.

The new value is not displayed, to anyone, ever.

### `sealbox-cli runner --name <name>`

Claims jobs addressed to this runner, executes them, and reports back. **This is the only place a
grant runs, and the only place a secret's plaintext exists outside the server.**

```bash
sealbox-cli runner --name prod-cluster
```

It dials out — the network it sits in needs no inbound port, no Ingress, and no public endpoint.

A claim carries the implementation and the plaintext of **only** the secrets that grant declares.
There is no endpoint, for any role, that fetches a secret by name.

**Three injection forms**, from two fields on the grant:

| Declared as | The implementation receives |
|---|---|
| `secrets` | each as an environment variable |
| `secrets` | *also* all of them as a `KEY=value` file at `$SEALBOX_ENVFILE` — for `kubectl create secret --from-env-file` and the like |
| `files` | each written to a `0600` file; the path is exported and substituted into argv as `{NAME}` |

Everything lives in one temp directory removed when the job ends, including on failure.

A job that is claimed and never reported is marked failed after ten minutes. **Nothing is
retried**: a grant is not necessarily idempotent, and silently re-running a `CREATE USER` or a
deployment is worse than failing. Resubmit when you have decided that is safe.

---

## Setup and recovery *(target)*

### `sealbox-cli init`

One-time initialisation of a fresh server: generates the recovery keypair locally, has the server
generate its master key, and forces you to type the recovery key back before finishing. **Does not
exist yet** — today, `bootstrap` claims the server and the server's master key file is the backup.

### `sealbox-cli recovery-export` / `recovery-restore`

Exports the encrypted recovery blob — safe to store anywhere, because it is encrypted to a key the
server does not hold — and rebuilds a server's master key from it. **Neither exists yet.**

---

## Configuration

Server URL and identity token live in `~/.config/sealbox/config.toml`; see
[Configuration](configuration.md). Admin operations read no credential from that file, because
none exists.
