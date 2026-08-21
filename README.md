# Sealbox

[![CI](https://github.com/realmorrisliu/sealbox/workflows/CI/badge.svg)](https://github.com/realmorrisliu/sealbox/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org)

> An isolation layer between agents and real credentials.
> **An agent can use a credential without ever seeing it.**

> ### ⚠️ Status
>
> Sealbox is being rebuilt around the design in [`docs/agent-native-design.md`](docs/agent-native-design.md).
> **The documentation describes the target system; the code does not implement it yet.** The
> commands below are the intended interface, not a working install. The previous generation — a
> single-token secret store with a React UI — is still what compiles, and is being removed rather
> than extended.

## Why

Give an agent a credential and it enters the model's context, the transcript, and the provider's
logs. Worse, agents bring a threat ordinary secret managers never had to model: **prompt
injection**. An agent reads a hostile issue and is talked into exfiltrating the token it just
fetched. Encryption at rest does not help.

Two rules, and everything else follows from them:

- **No interface returns a secret's value.** There is no `get_secret`.
- **An agent cannot compose a command.** It invokes *grants* a human approved.

A **grant** is a permitted use of secrets — which secrets, what is done with them, where it runs.
The relationship reads best backwards:

```console
$ sealbox ls --uses pg/prod-admin-password
pg-provision
rotate-utopia-db
```

**Two lines: everything that credential can do in this system.** In Vault, 1Password, or GitHub
Secrets the answer is "anything, to anyone holding it". Collapsing *unbounded* into *a list you
can read* is the product.

## How it fits together

```
sealbox-server (hosted)      store · authorise · generate values · dispatch · audit
                             never executes anything, cannot reach your VPC
      ↑ long-poll (outbound)       ↑ HTTPS
sealbox runner (your cluster)   sealbox CLI (laptop / agent)
  executes; the only place        submits jobs, shows results
  plaintext exists outside        never receives plaintext
  the server
      ↓
your infrastructure          cluster API · databases · cloud APIs
```

The server is hosted and has no route into your VPC — deliberately, because giving it one would
mean exposing production to the internet. The runner **polls outbound**, so your cluster opens no
inbound port. The CLI is a remote control.

## What it looks like

```bash
# A human grants a capability — approved with a passkey, in a browser or on a phone
sealbox grant add ./grants/k8s-sync.toml

# An agent uses it — and learns nothing
sealbox run k8s-sync ns=production
sealbox rotate pg/app/database-url --via pg-provision --from-output host=... user=app

# Afterwards
sealbox audit --since 24h
```

```toml
# grants/k8s-sync.toml — what a human approves is this, not a script
[k8s-sync]
adapter = "kubernetes-secret"
runner  = "prod-cluster"
config  = { namespace = "production", name = "app-runtime-secrets" }
secrets = { DATABASE_URL = "app/database-url", OSS_ENDPOINT = "app/oss-endpoint" }
```

The adapter is structurally incapable of anything but writing a Secret. A script holding the same
access could `delete ns production`.

## Design

- **No plaintext leaves the server** except into a runner executing a grant.
- **Humans authenticate with passkeys.** No admin credential exists on disk. The approval page is
  server-rendered, which makes it a *trusted display* — a terminal cannot be one, because its
  output is written by whatever process is running.
- **Rotation creates a second credential** and drops the old one only after verification. There is
  never a moment when no working credential exists.
- **Provider logic lives in adapters or scripts, never in sealbox.** Two adapters ship:
  `kubernetes-secret` and `postgres-role`. An adapter is only built in once it would replace two
  scripts that actually exist.
- **SQLite, one file.** Writes are rare, the dataset is megabytes, Litestream covers durability.

Every decision is recorded with its reasoning in [`docs/adr/`](docs/adr/) — including the ones
that were reversed.

## Documentation

| | |
|---|---|
| [Design](docs/agent-native-design.md) | Topology, secret lifecycle, security boundary, MVP |
| [Decisions](docs/adr/) | 12 ADRs, each with what was rejected and why |
| [Glossary](CONTEXT.md) | The vocabulary, and the synonyms to avoid |
| [Getting started](docs/getting-started.md) | The intended setup, once it exists |
| [CLI reference](docs/cli-reference.md) | Command surface |
| [Configuration](docs/configuration.md) | Server, runner, and CLI configuration |

## What sealbox is not

Not a smaller Vault. Vault, External Secrets Operator, and Sealed Secrets all solve storage and
delivery well, and all cost real adoption effort — unsealing and policy HCL, operators and CRDs,
controllers and rotation rituals. Sealbox is cheap to adopt, but that is not the moat: anything
that is merely "a simpler X" loses its users as they grow.

What holds is the part the others cannot do at all — **credentials that agents use without seeing,
confined to commands a human wrote down.**

## Building

```bash
cargo build --release
cargo test --workspace
cargo clippy --all-targets --all-features --workspace -- -D warnings
```

## License

Apache 2.0 — see [LICENSE](LICENSE).
