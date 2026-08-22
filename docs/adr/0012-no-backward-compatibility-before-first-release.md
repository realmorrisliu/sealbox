# No backward compatibility before the first release

Until sealbox has a user other than its author, nothing is preserved for the sake of an older
version: not database schemas, not API shapes, not configuration names, not CLI commands. There
are no migrations, no deprecation periods, and no compatibility shims.

The previous generation has no deployments. Writing a migration for it means writing, testing,
and then carrying code whose only purpose is to serve a database nobody has — and every such
path is one more thing that must keep working, be reasoned about during the next change, and be
read by whoever comes next.

This is a decision about *when*, not a permanent stance. The moment there is a deployment that
is not the author's, this reverses: from then on, a schema change ships with a migration and an
interface change ships with a transition.

## Consequences

An incompatible database is not migrated; it is deleted and rebuilt. `docs/` documents the
current shape only, with no upgrade notes.

The freedom is real and worth using deliberately: renaming a field, restructuring a table, or
changing a command's arguments costs nothing right now and will cost a great deal later.
Decisions that are being deferred because they would be breaking should be made *now*, not
after the first user arrives.

Anything already recorded in `openspec/specs/` still holds — those are statements about how the
system behaves, not promises to an installed base, and they change through a change proposal
rather than by drift.

## The cryptographic construction is not covered by this

`secret-encryption` requires that a dependency upgrade not alter the stored format or the
parameters of the encryption. That requirement stands, and this ADR does not relax it, because
the two are about different failures.

Breaking a schema is loud: the database is rebuilt and the cost is a few minutes. Breaking a
cryptographic construction can be silent — a changed nonce derivation, a different padding
mode, a weaker default — and produces data that still appears to work while being less safe
than intended. That is not something to discover after there are users.

So: schemas, APIs, configuration, and commands may break freely. What the ciphertext is and how
it was produced may not change without a deliberate, reviewed decision.
