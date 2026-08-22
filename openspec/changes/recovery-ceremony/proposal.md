# The backup stops being a manual `cat`

## Why

The server's master key is the only thing that can read the store, and it exists in exactly one
place: `/data/master.pem` on the volume. Litestream replicates the database and not the key, so
losing the volume means the replicated database is ciphertext under a key that no longer exists —
a healthy-looking backup that decrypts to nothing.

Today the operator copies the key off by hand, once, immediately after the first deploy. By
[ADR 0013](../../../docs/adr/0013-automation-first.md)'s measure that is the last thing left in
sealbox that a person maintains because nobody built the alternative — and it is a manual step in a
security-critical position, which is the kind people skip.

## What changes

[ADR 0010](../../../docs/adr/0010-recovery-via-keypair-not-a-copied-key.md) already says what to
build: a recovery keypair generated locally, the master key stored encrypted under its public half,
and the private half kept by the operator. This builds it.

- `recovery init` — the CLI generates a keypair, keeps the private half, and uploads the public
  half. The server encrypts its master key under it and stores the result as a **recovery blob**.
- The blob is re-made **automatically** whenever the master key changes, so the backup cannot go
  stale without anyone noticing.
- `recovery export` — fetch the blob. Safe to store anywhere: it is encrypted to a key the server
  does not hold.
- `recovery restore` — turn a blob and the recovery key back into `master.pem`, with no server
  involved.

No new mechanism. A recovery key is a master key with `server_held = 0` — the cold path from
[ADR 0001](../../../docs/adr/0001-broker-over-e2ee.md), reused rather than reinvented.

## Where this departs from ADR 0010

The ADR says initialisation "does not complete until the operator enters it back", modelled on a
1Password Emergency Kit or a hardware wallet's seed phrase. That works because those are short.
**A recovery key here is a 1.7 KB PEM.** Nobody transcribes twenty-five lines of base64, and a
ceremony that asks them to is a ceremony they will paste around or skip.

The property the re-entry was buying is *an unverified backup is not a backup*. So verification
does the stronger thing directly: the CLI takes the file it just wrote, fetches the blob the server
stored, and **decrypts it**, refusing to report success until the artefact the operator now holds
has actually recovered the master key. That checks the whole chain rather than a transcription, and
it is what the operator will do at three in the morning anyway.

## Non-goals

- **Passphrase-derived recovery keys.** They would be short enough to write down, but the server
  could no longer re-make the blob on its own after a rekey, which is the automation this exists
  for. A file in a password manager beats a phrase the backup goes stale behind.
- **Uploading anything the server could decrypt with.** Only the public half is sent, and the
  server keeps no copy of the private one — that is what makes the blob safe to store anywhere.
