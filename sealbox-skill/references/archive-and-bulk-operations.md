# Archive and bulk operations

Sealbox supports encrypted bulk operations for backup and migration through CLI import/export.

## `secret export`

- Creates archive files from current local-secret records.
- Exported artifacts are encrypted in structure and can be versioned back.
- Useful for:
  - off-host backup
  - migration to a fresh DB
  - cross-environment transfer workflows

## `secret import`

- Loads a compatible archive into current server context.
- Can restore encrypted versions and metadata according to import format.
- Requires appropriate target key material and endpoint compatibility.

## Supported archive formats

- `sealbox-v1`: format for v1-era payload compatibility.
- `encrypted-tar`: encrypted tar-based portable bundle.
- Archive contains manifest and encrypted secret payloads; it is not plaintext JSON export.

## Safety notes

- Treat exported files as sensitive secrets-bearing artifacts.
- If importing into tenant mode, validate tenant mapping and API version.
- Rehearse import/export in a staging environment before production rotations.
- Verify key state (`key status`) and active key IDs after import.
