# Design

## The blob is envelope encryption, unchanged

RSA-OAEP cannot encrypt 1.7 KB directly, and inventing something for this would be inventing
cryptography. The blob is exactly what a secret is: a random data key encrypting the payload, and
the data key encrypted under the recovery public key. The payload is the master key file's bytes.

Stored in its own table rather than in `secrets`, because a secret is something grants may declare
and listings show, and this is neither. Same code path, different home.

## Kept current without anyone asking

Whenever the server's master key changes — a new one created, or a rekey moving secrets onto it —
the blob is re-made under every registered recovery key. A backup that silently stops matching what
it is meant to restore is worse than no backup, and remembering to refresh it is exactly the kind
of task ADR 0013 says should never be a person's.

## Restore needs no server

```bash
sealbox-cli recovery restore --blob ./blob.json --key ./recovery.pem --out ./master.pem
```

It reads a file, decrypts, writes a file. That matters: recovery happens when the server is gone,
so a restore path that needs one is not a restore path. The blob is fetched from a healthy server
by `recovery export`, or from wherever the operator put it.

## More than one recovery key

Registering a second does not replace the first; both get a blob. An operator who loses a key while
the server is healthy re-registers a new one rather than being locked out, and two people can each
hold one without sharing.

Removing one is a separate act, so that "add the new key" and "retire the old key" cannot be
confused for each other.

## What the server may never do

- Return the master key from any endpoint, in any form other than encrypted under a recovery key.
- Log it, at any level.
- Keep a recovery private key. Only the public half arrives, and nothing stores more than that.
