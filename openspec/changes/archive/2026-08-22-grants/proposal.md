## Why

A secret's value is protected, but what it may be *used for* is not expressed anywhere. Anyone
who can read a secret can do anything the credential permits — which is the same unbounded
authority every other secret manager grants, and the thing sealbox exists to narrow.

A **grant** is a permitted use: which secrets it needs, what is done with them, and where it
runs. Read the relationship backwards and it is the product:

```console
$ sealbox ls --uses pg/prod-admin-password
pg-provision
rotate-utopia-db
```

Two lines — everything that credential can do in this system. In Vault, 1Password, or GitHub
Secrets the answer is "anything, to anyone holding it". Collapsing *unbounded* into *a list you
can read* is what the rest of the design is in service of.

Grants also make the agent boundary enforceable rather than aspirational. An agent invokes a
grant by name; it cannot compose a command, and it cannot approve a new one
([ADR 0003](../../../docs/adr/0003-named-grants-not-free-form-commands.md)). Withholding the
plaintext alone does not survive prompt injection — an interface shaped
`run_with_secrets(secrets, command)` still lets a compromised agent do anything the credential
permits, including piping data to an attacker. Withholding *command composition* is the other
half.

## What Changes

- Add **grants**: a name, an implementation (a named built-in adapter or a stored script), the
  runner that will execute it, the secrets it declares, and optionally a linear `then` chain.
- **Approval is the gate**: creating a grant requires the admin role. An agent can draft one; it
  cannot make it runnable.
- **Script bodies are stored, never referenced by path.** A grant pointing at a file could be
  approved once and the file edited afterwards, so what was reviewed and what runs would differ.
- Add `sealbox ls --uses <secret>`: every grant that declares a given secret. This is the query
  that makes a credential's authority legible.
- Validate at approval time, when a human is present to see the failure: the declared secrets
  exist, the named adapter is known, and a `then` chain neither references a missing grant nor
  forms a cycle.
- Add `sealbox-cli grant add|list|show|rm`.

Explicitly **not** in this change: executing anything. Adapters are recognised by name and their
configuration validated, but their implementations arrive with the runner that will run them —
there is nowhere to execute a grant until then.

## Capabilities

### New Capabilities

- `grant`: what a permitted use of secrets is, who may create one, what is checked before it
  becomes runnable, and how a secret's authority can be enumerated.

### Modified Capabilities

None.

## Impact

**Implements** the definition half of MVP item 4. **Blocks** `jobs-and-runner`, which executes
what this defines, and `rotation`, which is a grant invoked in a particular way.

**Constrained by** ADR 0003 (agents invoke named grants and never compose commands), ADR 0007
(adapters preferred, scripts as the escape hatch, and sealbox never generates secret material
inside an implementation), and ADR 0009 (approval will move to a passkey; nothing here may
assume the admin credential is a token).

**Code**
- `sealbox-server/src/repo/` — a `grants` table and its repository
- `sealbox-server/src/api/handler/` — grant endpoints, admin-gated except listing
- `sealbox-server/src/api/handler/secret.rs` — `--uses` reverse lookup
- `sealbox-cli/src/commands/` — `grant` subcommands, and grant files parsed as TOML

**Interfaces** — new endpoints only; nothing existing changes shape.

**Security** — this is where a human's judgement enters the system. What is reviewed is the
capability declaration, not the script: sealbox confines an implementation to exactly the
secrets it declares, so however it is written it cannot reach anything else. Judging whether a
shell script is safe is a hard cognitive task, and that kind of review reliably decays into a
glance; judging one line does not.
