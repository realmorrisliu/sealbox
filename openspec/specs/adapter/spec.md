# adapter Specification

## Purpose
A built-in implementation of a grant for one class of target system. Adapters exist because a
script can do anything its declared secrets permit and an adapter cannot — that bound is what
makes approving a grant a matter of reading one declaration rather than auditing code.
## Requirements
### Requirement: An adapter's configuration cannot widen what it does

An adapter SHALL accept only structured parameters, and SHALL NOT accept a command, a script, a
query, a resource kind, or a verb from its configuration.

The system SHALL reject an adapter configuration containing a field it does not define.

#### Scenario: An unknown configuration field is refused

- **WHEN** a grant configures an adapter with a field that adapter does not define
- **THEN** the system refuses the grant at creation

#### Scenario: Configuration cannot express a different operation

- **WHEN** an adapter's configuration is examined
- **THEN** no value in it can cause the adapter to perform an operation other than the one it
  implements

### Requirement: Adapter configuration is validated when the grant is approved

The system SHALL validate an adapter's configuration at grant creation, and SHALL name what is
wrong.

#### Scenario: A missing required setting

- **WHEN** a grant names an adapter without a setting that adapter requires
- **THEN** the system refuses the grant and names the missing setting

#### Scenario: A value outside the permitted set

- **WHEN** a configuration value falls outside what the adapter permits
- **THEN** the system refuses the grant and says what is permitted

### Requirement: The kubernetes-secret adapter writes exactly one Secret

The `kubernetes-secret` adapter SHALL write the grant's declared secrets into a single named
Kubernetes Secret in a single named namespace, using the runner's own credentials.

It SHALL NOT delete, read, or modify any other resource, and SHALL NOT act on any other resource
kind.

#### Scenario: Synchronising secrets

- **WHEN** a grant using this adapter runs
- **THEN** the named Secret in the named namespace contains each declared secret as a key
- **AND** no other resource is touched

#### Scenario: Existing Secret is replaced, not merged

- **WHEN** the named Secret already exists
- **THEN** its contents become exactly the grant's declared secrets, so that removing a secret
  from the grant removes it from the cluster

### Requirement: The postgres-role adapter creates a role rather than changing one

The `postgres-role` adapter SHALL create a new role whose password is the value provided to it,
and SHALL emit a connection URL for that role.

It SHALL NOT alter the password of an existing role, and SHALL NOT drop any role.

#### Scenario: Rotating without a window of failure

- **WHEN** a rotation using this adapter runs
- **THEN** a new role exists with the new password, and the previous role still works
- **AND** both remain usable until something else removes the older one

#### Scenario: The emitted URL is the value

- **WHEN** the adapter is used with capture
- **THEN** it emits a connection URL containing the new role and its password, and nothing else

#### Scenario: The password is percent-encoded

- **WHEN** the emitted URL is constructed
- **THEN** the password is encoded so that the URL parses correctly whatever characters it
  contains

### Requirement: Privileges come from a closed set

The `postgres-role` adapter SHALL grant only privileges named in its configuration, and SHALL
reject any privilege outside a fixed, documented set.

#### Scenario: An unrecognised privilege is refused

- **WHEN** a grant configures a privilege outside the permitted set
- **THEN** the system refuses the grant at creation, and names what is permitted

#### Scenario: No privilege is granted implicitly

- **WHEN** a role is created
- **THEN** it holds only the privileges the configuration named

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

