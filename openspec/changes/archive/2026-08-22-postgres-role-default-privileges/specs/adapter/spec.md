## ADDED Requirements

### Requirement: A provisioned role reaches objects created after it

A role-provisioning adapter SHALL grant its declared privileges on the objects that exist when it
runs **and** on the objects the declared owner creates afterwards.

The grant SHALL declare that owner, and the system SHALL refuse one that does not.

#### Scenario: A role provisioned before migrations still works after them

- **WHEN** a role is provisioned into an empty database and the owner then creates tables
- **THEN** the role has its declared privileges on those tables

#### Scenario: A role provisioned after migrations works immediately

- **WHEN** a role is provisioned into a database that already has tables
- **THEN** the role has its declared privileges on them

#### Scenario: A grant that names no owner is refused

- **WHEN** a role-provisioning grant declares no owner
- **THEN** the system refuses it at approval, rather than creating a role that breaks on the next
  migration

### Requirement: A declared write privilege works on ordinary tables

Where a declared privilege requires access to a supporting object to function — inserting into a
table whose key comes from a sequence — the adapter SHALL grant that access.

#### Scenario: Inserting works without a further grant

- **WHEN** a role declaring `INSERT` inserts into a table with a generated key
- **THEN** it succeeds, rather than failing on an object the grant never mentioned
