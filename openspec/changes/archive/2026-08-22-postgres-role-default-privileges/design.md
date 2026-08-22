# Design

## What the adapter emits

```sql
BEGIN;
CREATE ROLE app_1 LOGIN PASSWORD :'pw';
GRANT CONNECT ON DATABASE app TO app_1;
GRANT USAGE ON SCHEMA public TO app_1;

-- what exists now
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_1;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO app_1;

-- what the owner creates later
ALTER DEFAULT PRIVILEGES FOR ROLE migrator IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_1;
ALTER DEFAULT PRIVILEGES FOR ROLE migrator IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_1;
COMMIT;
```

Still one transaction, so a failure anywhere leaves no half-made role.

## Why sequences are not a separate decision

A grant declaring `INSERT` is declaring that the role inserts rows. A table with a `serial` column
takes a value from a sequence to do that, and without `USAGE` the insert fails at runtime with an
error about an object the grant never mentioned. Granting it is not widening what was approved; it
is making the declared privilege work.

`SELECT` on the sequence comes with `USAGE` because `currval`/`lastval` are ordinary in the same
code paths, and a sequence's value is not a secret — it is a row count with a lead.

Identity columns (`GENERATED … AS IDENTITY`) need none of this; the grants are harmless there.

## Why not infer the owner

The connecting admin could be assumed to be the owner, and it often is. But when it is not — the
common case, where migrations run as their own role — the default privileges would silently attach
to the wrong role and produce exactly the failure this change exists to remove, with no error
anywhere. A guess that is usually right and silently wrong the rest of the time is worse than a
required field.

## Validation

`owner` reaches SQL as an identifier, so it is checked the same way `role_prefix` is: letters,
digits, and underscores. Checked at approval, while a person is present.
