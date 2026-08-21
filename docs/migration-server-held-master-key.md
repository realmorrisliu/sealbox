# Migration: the server holds its own master key

> One-time upgrade for a sealbox deployment that predates the server-held master key. On a fresh
> install none of this applies — just set `SEALBOX_MASTER_KEY_PATH` and start.

This release removes `PUT /v1/master-key`'s `old_private_key_pem` field, which required clients
to transmit a private key in the clear. In its place the server holds a master key of its own.
Two schema changes come with it, applied automatically on first start.

## What changes on disk

| | |
|---|---|
| `secrets` | Rebuilt without the unused `namespace` column; primary key becomes `(key, version)` |
| `master_keys` | Gains `server_held`, defaulting to `0` |

Both migrations are idempotent, and the `secrets` rebuild compares row counts before and after,
failing loudly rather than proceeding if they differ.

Environment variables also gain a `SEALBOX_` prefix: `STORE_PATH` → `SEALBOX_STORE_PATH`,
`AUTH_TOKEN` → `SEALBOX_AUTH_TOKEN`, `LISTEN_ADDR` → `SEALBOX_LISTEN_ADDR`.

## Steps

**1. Back up the database. It is the only copy.**

```bash
cp /var/lib/sealbox.db /var/lib/sealbox.db.backup
```

**2. Generate the server's master key.**

```bash
openssl genrsa -out /var/lib/sealbox-master.pem 2048
chmod 600 /var/lib/sealbox-master.pem
```

Sealbox will not create this for you. A mistyped path would silently generate a fresh key,
leaving every stored secret encrypted under one nobody holds — a failure that would only surface
later, on a read.

**3. Start with the new configuration.**

```bash
SEALBOX_STORE_PATH=/var/lib/sealbox.db \
SEALBOX_AUTH_TOKEN=... \
SEALBOX_LISTEN_ADDR=127.0.0.1:8080 \
SEALBOX_MASTER_KEY_PATH=/var/lib/sealbox-master.pem \
  sealbox-server
```

The log will show the migration and the key registration:

```
Migrating `secrets`: dropping the unused `namespace` column
Migrated N secrets
Server master key <uuid> loaded from /var/lib/sealbox-master.pem (current)
```

**4. Verify.**

```bash
sqlite3 /var/lib/sealbox.db "select count(*) from secrets;"          # unchanged
sqlite3 /var/lib/sealbox.db "select server_held from master_keys;"   # 1 for the new key
curl -s localhost:8080/healthz/ready                                  # ready
```

## Your existing secrets are not migrated, and cannot be

Every master key registered before this change was submitted as a **public key only** — the
server never had the private half. Under the new model all of them are therefore **cold**: the
server cannot decrypt secrets encrypted under them, and rekey deliberately refuses a cold source.

That refusal is the point of this release. Accepting one would mean re-adding the endpoint the
release exists to remove.

So existing secrets stay readable exactly as before — by a client holding the corresponding
private key — but the server cannot serve or rekey them. To bring one onto the server-held key,
read it with that client and store it again:

```bash
sealbox-cli secret get my-secret            # decrypts locally with your private key
sealbox-cli secret set my-secret "<value>"  # re-stored under the server-held key
```

New secrets use the server-held key automatically.

## Rotating the server's master key later

`SEALBOX_MASTER_KEY_PATH` accepts a comma-separated list, most-current first. Keep the old key
listed while rekeying, or the private half needed to read the existing secrets will already be
gone:

```bash
SEALBOX_MASTER_KEY_PATH=/var/lib/master-2.pem,/var/lib/master-1.pem
```

Rekey, confirm nothing still references the old key, then drop it from the list.

## Rollback

Restore the backup and revert the binary. The schema migration is not backward-compatible, so
rollback is at the file level. Nothing outside sealbox reads these tables.
