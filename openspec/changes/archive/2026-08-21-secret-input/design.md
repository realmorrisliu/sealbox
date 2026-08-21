## Context

See proposal.md — Why.

`PUT /v1/secrets/{key}` takes `{ "secret": "...", "ttl": ... }` and the server does the envelope
encryption. `Secret::new` already generates a random data key per secret, so a CSPRNG and the
encryption path both exist; what is missing is a way to ask for the *value* to be generated too.

The CLI takes the value as an optional positional argument, falling back to stdin. It also
claims listing is unsupported while `GET /v1/secrets` works.

## Goals / Non-Goals

**Goals:**

- A secret's value cannot be passed as a command-line argument.
- The server can generate a value that never crosses the network.
- Listing states what it returns, and returns it.

**Non-Goals:**

- Removing `secret get`. It hands ciphertext to a client that decrypts locally, which the target
  design replaces — but nothing else can consume a secret until runners exist. Removing it now
  would make the system unusable rather than safer. It goes with `jobs-and-runner`.
- Key derivation, passphrase generation from wordlists, or format-preserving values. Two shapes
  cover what the acceptance scenario needs; more can be added when something needs them.
- Changing the stored format or the cryptographic construction (`secret-encryption`).

## Decisions

### One endpoint, two payload shapes

`PUT /v1/secrets/{key}` accepts either a supplied value or a generation request, distinguished
by which field is present:

```json
{ "secret": "..." }                              // supplied
{ "generate": { "type": "password", "length": 32 } }   // generated
```

Rather than a second endpoint, because the outcome is identical — a new version of that secret —
and the only difference is where the bytes came from. A separate `POST /secrets/{key}/generate`
would duplicate versioning, TTL handling, and the master key lookup.

The two are mutually exclusive and unknown fields are rejected, so a request that supplies both
fails rather than silently preferring one.

### Generation happens in the repository, next to encryption

The value is generated where `Secret::new` already runs, so the plaintext exists only inside
that call: generated, encrypted, dropped. It is never assigned to a field, returned, or logged.

*Alternative rejected:* generating in the handler and passing the value down. That widens the
plaintext's lifetime across a layer boundary for no benefit.

### A minimum length, enforced rather than documented

Generation refuses a length below the minimum instead of honouring it. A caller asking for an
8-character password is more likely to have made a mistake than to have a reason, and the cost
of being wrong is a weak credential that looks exactly like a strong one.

Defaults: 32 characters for `password`, 32 bytes for `hex`. The minimum is 16 for both.

### `password` excludes ambiguous characters, `hex` does not

`password` draws from an alphabet without `0`/`O`, `1`/`l`/`I`. These values get read aloud,
retyped from a screenshot, and pasted into places that mangle them. The entropy lost is small
and the length compensates; the confusion avoided is real.

`hex` is for machine consumption and stays exactly what it says.

### `set` reads stdin, and only stdin

Not "stdin unless an argument is given". The argument form is removed outright: while it exists,
it will be used, and every use puts a credential into shell history and `ps` output.

Bulk import stays a shell loop, per the design's rejection of an import command.

## Risks / Trade-offs

- **Removing the argument form breaks existing muscle memory** → Intended, and cheap under
  ADR 0012. The error names stdin explicitly rather than failing on a missing argument.
- **A generated value cannot be retrieved by the caller who asked for it** → Deliberate: it is
  what lets an agent provision a credential it cannot read. Anything that needs to *use* it does
  so through a grant. Until runners exist, a generated secret is write-only in practice, which
  is why generation lands before grants rather than after.
- **The minimum length will occasionally be inconvenient** → It is a floor, not a policy engine.
  Raising or lowering it is a one-line change with a recorded reason.

## Migration Plan

None. `PUT /v1/secrets/{key}` accepts what it accepted before, plus a second shape.

**Rollback:** revert the binary; stored secrets are unaffected either way.

## Open Questions

None.
