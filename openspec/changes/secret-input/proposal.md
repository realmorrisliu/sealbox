## Why

Two problems with how values get into sealbox today, both on the input side.

**A secret can be passed as a command-line argument.** `sealbox-cli secret set mykey "hunter2"`
puts the value in shell history, in `ps` output for every user on the machine, and in any shell
integration that records commands. The CLI already reads stdin when the argument is omitted, so
the unsafe path exists purely because it is possible.

**Nothing can be generated.** Every value has to be produced somewhere else and carried in,
which means it exists in at least one more place than necessary — a terminal, a clipboard, an
agent's context. For anything whose value is just a random number, that entire journey is
avoidable: the server can generate it and encrypt it without the plaintext ever leaving.

Generation is also what makes an agent able to *provision* rather than only consume. An agent
can ask for a database password to be created without ever being able to read it.

## What Changes

- **BREAKING** — `secret set` no longer accepts a value as an argument. The value is read from
  stdin, always.
- Add generation: the server produces the value from a CSPRNG, encrypts it, and stores it. The
  plaintext never crosses the network in either direction.
  - `sealbox-cli secret gen <key> --type password|hex [--length N] [--ttl N]`
  - Server-side, via the existing `PUT /v1/secrets/{key}` with a payload that asks for
    generation instead of supplying a value.
- `secret list` reports what the server actually returns. It currently claims listing is
  unsupported while `GET /v1/secrets` exists and works.
- Listing returns metadata only — key, version, timestamps, expiry — never a value, and this
  becomes a stated requirement rather than an accident of implementation.

Explicitly **not** in this change: removing `secret get`. It returns ciphertext for the client
to decrypt, which the target design does away with — but nothing can use a secret until runners
exist, so removing it now would make the system unusable rather than safer. It goes with
`jobs-and-runner`.

## Capabilities

### New Capabilities

- `secret`: how a secret's value comes into existence and is stored — supplied or generated —
  what versioning and expiry mean, and what listing is allowed to reveal.

### Modified Capabilities

None.

## Impact

**Implements** MVP item 3. **Blocks** `grants-and-adapters`, which needs generated values to be
possible before rotation can mean anything.

**Constrained by** ADR 0007 (sealbox generates values; implementations never produce secret
material) and `secret-encryption` (the stored format and the cryptographic construction do not
change here).

**Code**
- `sealbox-server/src/api/handler/secret.rs` — the save payload gains a generated variant
- `sealbox-server/src/repo/` — a way to store a generated value alongside the supplied one
- `sealbox-cli/src/commands/secret_commands.rs` — `set` from stdin only, `gen` added, `list`
  corrected

**Interfaces** — `PUT /v1/secrets/{key}` accepts a second payload shape. Passing a value as a
CLI argument stops working, which is the point.

**Security** — this removes the most common way a secret ends up somewhere it should not be: a
shell history file. Generation removes a second one, by never letting the value exist outside
the server at all.
