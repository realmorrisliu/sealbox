# A provisioned role can read the tables that come later

## Why

`postgres-role` grants a new role its table privileges like this:

```sql
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_1;
```

`ON ALL TABLES` covers the tables that exist **at that moment**. Nothing it grants applies to a
table created afterwards.

Which breaks the ordinary order of operations:

```
provision the runtime role      → the database is empty, so this grants nothing
migrations run as the owner     → tables appear
the service connects            → permission denied for every one of them
```

This does not show up in a test, and it did not show up when the adapter was verified against a
real Postgres: the database had no tables, the `GRANT` returned success, the role connected, and
everything looked correct. It only appears in the migrate-then-run order that every real
deployment has.

## What changes

- A grant declares the **owner** — the role that creates objects — and the adapter also issues
  `ALTER DEFAULT PRIVILEGES FOR ROLE <owner>`, which is what covers tables created later.
- Both are issued, not one: `ON ALL TABLES` for what already exists, default privileges for what
  comes next. A role provisioned before migrations and one provisioned after must end up the same.
- Sequences come with it. `INSERT` into a table with a `serial` column fails with *permission
  denied for sequence* without `USAGE`, which is the same class of error one layer down.
- `owner` is **required**. A grant without one produces a role that works until someone migrates,
  and that is a worse failure than being told to name it.

## Why the owner has to be named

Default privileges attach to the role that *creates* the object, not to the schema. Postgres has
nowhere to record "whoever creates tables here, grant to that role" — it can only record "when
`<owner>` creates a table, grant to `<role>`". So the grant has to say who that is.

## Cost, stated plainly

`ALTER DEFAULT PRIVILEGES FOR ROLE <owner>` requires the connecting account to be a member of
`<owner>`, or a superuser. On a managed instance whose privileged account is not a true superuser,
that may need `GRANT <owner> TO <admin>` once. If it fails, the whole provisioning transaction
rolls back and nothing is left half-made — but the operator has to know why, so the failure says
so.
