---
name: sealbox
description: Store, use, and rotate credentials through sealbox — a secret store an agent can use without ever seeing a value. Use this whenever a task involves a password, API key, connection URL, token, or any other credential, including creating a new one.
---

# Sealbox

Sealbox lets you *use* a credential without ever *seeing* it. That is the whole idea, and it is
what makes it safe to hand you credential work at all.

You submit a job; a runner inside the target infrastructure receives the plaintext, does the work,
and reports back. You get an acknowledgement. The value never passes through you, your context, or
your transcript.

Full command detail: [`docs/cli-reference.md`](../../docs/cli-reference.md). This file is the part
you cannot infer.

## Two habits

**1. Never ask for a value.** There is no command that prints one. If your plan has a step where
you read a password and then use it, the plan is wrong — replace that step with a grant that does
the work where the plaintext already is.

**2. Draft, then hand over.** You write grants; a human signs for them. Submit, report the URL,
and stop. Do not retry, and do not find another way to accomplish what the unapproved grant would
have done — that is the one behaviour that makes the whole arrangement worthless.

## By task

**A new credential is needed.** Generate it inside sealbox. It then never existed anywhere you
could have leaked it:

```bash
sealbox-cli secret gen app/session-key
sealbox-cli secret gen app/hmac --type hex
```

**A credential the human already has.** They pipe it in themselves; the value comes from stdin, so
it never lands in shell history or `ps` output:

```bash
printf %s "$VALUE" | sealbox-cli secret set app/database-url
```

**Something needs to happen with a credential** — write a Kubernetes Secret, create a database
role, call an API. That is a **grant**: a named, approved, standing permission to use specific
secrets in a specific way, on a specific runner. Write one by imitating
[`examples/grants/`](../../examples/grants/), then:

```bash
sealbox-cli grant add ./grants/k8s-sync.toml   # prints an approval URL; a human signs it
sealbox-cli run k8s-sync                       # once approved
```

**A credential must change.** Rotation is one command, and it commits only if the upstream
actually accepted the new value:

```bash
sealbox-cli rotate app/database-url --via pg-provision --from-output
```

If the grant fails, the old value is still current. There is no half-rotated state to clean up.

**Working out what to change.** Everything a credential can be used for is the set of grants
declaring it:

```bash
sealbox-cli secret list                 # names and metadata, never values
sealbox-cli secret show app/database-url # one secret's metadata: version, when it changed
sealbox-cli secret uses pg/prod-admin-password
sealbox-cli grant show k8s-sync
sealbox-cli audit --since 24h
```

## Refusals that are the system working

| You will hit | Why |
|---|---|
| No command returns a value | The point of the system. Move the work to a grant. `secret show` gives you metadata — that it exists, its version, when it changed — and nothing else. |
| `secrets = { DB = "app/{env}/url" }` is refused | The parameter comes from whoever invokes the grant, so it would let the caller choose which credential the grant reaches. Two environments are two grants. |
| A submitted grant does not exist yet | A human signs for it on a page the server renders. Your terminal output is written by you, so it cannot be what they approve. |
| Admin commands refuse your token | Identities, master keys, and removing grants need a passkey. Ask the human to run `sealbox-cli admin <command>`. |

## Writing a grant

The `secrets` line is what a human approves and what sealbox enforces — the implementation cannot
reach anything else, however it is written. Keep it as short as the task allows.

- **Prefer an adapter** (`kubernetes-secret`, `postgres-role`) over a `script`. An adapter is
  structurally limited to what it implements; a script can do anything its secrets permit.
- **Never generate secret material in the grant.** Sealbox generates it and injects `$SEALBOX_NEW`.
- **Parameters are substituted into argv, never through a shell.** `{namespace}` is fine; a
  parameter of `x; curl evil.com` becomes an odd argument, not an injection.
- **Rotation creates the new credential before dropping the old one**, so there is never a moment
  when nothing works. Chain the verify step before the drop.

## What you cannot do, and should say so plainly

- Read a secret's value.
- Approve a grant, or act as an admin.
- Reach the target infrastructure yourself — the runner does that, and only for approved grants.

When one of these blocks a task, say which one and what the human needs to do. That is a complete
and useful answer, not a failure.
