# Design

## Why not a parameter or a role

The obvious shape — metadata by default, ciphertext behind `?ciphertext=true` limited to operator
and above — was rejected. It invents a permission tier whose only purpose is to hand out material
that the endpoint should not be handling, and every tier is a thing that can be misconfigured into
being reachable. Nothing needs it: the runner receives plaintext from the server (`job.rs` decrypts
before dispatch), and rekey happens entirely server-side. The only consumer of ciphertext over HTTP
is the CLI command being removed.

## Why offline is the right place for the cold path

A cold secret's defining property is that the server cannot decrypt it. Reading one therefore has
nothing to do with the server, and routing it through the API costs three things:

- it only works while the server is healthy, and a cold secret is what you reach for when it is not;
- it puts ciphertext on a route that authorisation has to keep getting right forever;
- it makes "which secrets can an agent carry away in a form that is ever decryptable" a question
  with a non-obvious answer, when the product's entire claim is that such questions have short,
  readable answers.

An offline tool has none of those. It reads the SQLite file — a Litestream restore, a volume
snapshot, a copy — and a key file, and it works when everything else is down.

## Shape of the offline tool

Recorded so it is not redesigned from scratch later:

```
sealbox-recover --db ./sealbox.db --key ./cold.pem <secret-key> [--version N]
```

It refuses a secret whose `master_key_id` does not match the supplied key, rather than failing in
the decrypt with an opaque error. It is a separate binary, not a `sealbox-cli` subcommand: a
recovery tool that shares configuration loading with the everyday client is a recovery tool that
can fail for the same reason the everyday client is failing.

## What is deliberately not touched

`GET /v1/secrets/{key}` still exists and still 404s for an unknown key. Removing the endpoint would
break the "does this exist and when was it last rotated" question, which agents legitimately ask
and which reveals nothing.
