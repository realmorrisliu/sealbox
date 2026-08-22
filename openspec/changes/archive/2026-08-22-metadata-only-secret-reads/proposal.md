# Reads return metadata, never ciphertext

## Why

`GET /v1/secrets/{key}` returns the whole stored row — `encrypted_data` and `encrypted_data_key`
included — to any identity with the `agent` role. `sealbox-cli secret get` then decrypts it with a
master key held locally.

For a **cold** secret that is the intended read path (ADR 0001): the server cannot decrypt it, so
whoever holds the key does. For a **server-held** secret it is dead weight with a cost — any agent
can carry away ciphertext for every secret in the store and keep it. The value is only as far away
as the master key ever leaking, and the store's own claim is that an agent never gets that close.

The deeper problem is where the cold path was put. Reading a cold secret through the server makes
the server a distribution channel for material it is supposed to have no relationship with, and it
only works while the server is healthy — which is precisely when a cold secret is *not* needed. The
moment it is needed is the moment the server is gone.

## What changes

- Reads return metadata only: key, version, master key id, timestamps, expiry. No ciphertext, at
  any role, under any parameter. There is no tier to get it, because a tier would be a way.
- `sealbox-cli secret get` and its client-side decryption are removed.
- The cold path's reader becomes an **offline tool** operating on a database file and a key file,
  with no server involved. Specified here, built when it is first needed — see Non-goals.
- `secret export`, `secret import`, and `secret history` are removed. They are stubs from the
  previous generation that print "not supported" and do nothing; a command that exists and does
  nothing is worse than an absent one, because an agent will call it and believe the answer.

## Non-goals

- **The offline tool is not built here.** No cold secret exists yet, so nothing is stranded, and
  building a break-glass tool before there is anything to break glass for guarantees it is wrong.
  What this change does is stop the API from pretending to be one.

## Cost, stated plainly

Until that tool exists, a cold secret can be written and not read back. That is acceptable only
because there are no cold secrets and no users. It stops being acceptable the moment someone
registers a master key the server does not hold, and this must be built before then.
