# Recovery uses a recovery keypair, not a copy of the server master key

At initialisation the CLI generates a **recovery keypair locally**. The public half is uploaded;
the server generates the server master key and stores it encrypted under that public key as a *recovery blob*.
The private half is displayed once, and initialisation does not complete until the operator
enters it back for verification.

The master key itself is never printed, logged, or returned by any endpoint. Logs get shipped,
aggregated, retained, and read by people who should not hold the key to every credential in the
system; a server master key that appears in `fly logs` is a server master key that has leaked.

**No new mechanism is introduced.** A recovery keypair is precisely a master key with
`server_held = 0` — the cold path from ADR 0001. Recovery reuses the code path that already
exists for credentials the server cannot decrypt.

## Mandatory verification

Initialisation does not finish until the backup has been shown to work, because an unverified
backup is reliably not a backup — it is a transcription error nobody discovers until the day it
matters.

**Amended when this was built.** The original form was re-entry: type the recovery key back, as
1Password's Emergency Kit and a hardware wallet's seed phrase do. That works because those are
short. A recovery key here is a 1.7 KB PEM, and nobody transcribes twenty-five lines of base64 —
a ceremony that asks them to is one they paste around or skip.

So verification does the stronger thing directly: the client takes the file it just wrote, fetches
the blob the server stored, and **decrypts it**, refusing to report success until the artefact the
operator now holds has actually recovered the master key. That checks the whole chain rather than a
transcription, and it is what the operator would do at three in the morning anyway.

## Consequences

The blob is re-made automatically whenever the server's master key changes. A backup that silently
stops matching what it is meant to restore is worse than no backup, and remembering to refresh it
is exactly the kind of task a person should never be holding (ADR 0013).

`sealbox-cli recovery export` can be run at any time and its output is safe to store anywhere,
because the blob is encrypted to a key the server does not hold. Losing the recovery private key
while the server is healthy is survivable — re-initialise recovery with a new keypair. Losing it
*and* the server is not, which is the property that makes verification non-negotiable.

Losing every registered passkey is also survivable: the recovery key decrypts the database
directly, independent of authentication. Authentication and encryption fail independently, by
design.
