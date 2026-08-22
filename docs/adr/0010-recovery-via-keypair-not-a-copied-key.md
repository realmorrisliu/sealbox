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

Initialisation forces the operator to re-enter the recovery key before finishing. This is the
1Password Emergency Kit / hardware-wallet seed-phrase convention, and it exists because an
unverified backup is reliably not a backup — it is a transcription error nobody discovers until
the day it matters.

## Consequences

`sealbox recovery-export` can be run at any time and its output is safe to store anywhere,
because the blob is encrypted to a key the server does not hold. Losing the recovery private key
while the server is healthy is survivable — re-initialise recovery with a new keypair. Losing it
*and* the server is not, which is the property that makes verification non-negotiable.

Losing every registered passkey is also survivable: the recovery key decrypts the database
directly, independent of authentication. Authentication and encryption fail independently, by
design.
