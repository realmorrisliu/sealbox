# grant Specification

## Purpose
A grant is a permitted use of secrets: which secrets it needs, what is done with them, and where
it runs. It exists so that a credential's authority is a list someone can read rather than
"anything the holder wants", and so that an agent can act without being able to choose what to
do.
## Requirements
### Requirement: A grant declares the secrets it may use

A grant SHALL declare, by name, every secret its implementation may access.

The system SHALL NOT make any secret available to a grant that the grant does not declare.

#### Scenario: Declaring secrets

- **WHEN** a grant is created declaring a set of secrets
- **THEN** that set is stored with the grant and is visible when the grant is shown

#### Scenario: Undeclared secrets are unreachable

- **WHEN** a grant's implementation attempts to use a secret it did not declare
- **THEN** that secret is not available to it

### Requirement: Declared secret names are literal

A declared secret SHALL be named literally. The system SHALL reject a grant whose declared secret
name contains a parameter placeholder.

Parameters supplied at invocation MAY appear in the implementation's arguments, but SHALL NOT
determine which secrets are available to it.

#### Scenario: A parameterised secret name is refused

- **WHEN** a grant is created declaring a secret whose name contains a placeholder
- **THEN** the system refuses it, so that no caller-supplied value can change which credential
  the grant reaches

#### Scenario: Two environments are two grants

- **WHEN** the same operation is needed against two sets of secrets
- **THEN** each is a separate grant naming its secrets literally, and each is approved separately

### Requirement: A secret's authority can be enumerated

The system SHALL be able to report every grant that declares a given secret.

#### Scenario: Asking what a credential can do

- **WHEN** the grants using a named secret are requested
- **THEN** the system returns every grant declaring it, and no others

#### Scenario: A secret no grant uses

- **WHEN** the grants using a secret that no grant declares are requested
- **THEN** the system returns an empty result rather than an error

### Requirement: Creating a grant requires the admin role

The system SHALL permit only an identity with the admin role to create, modify, or remove a
grant.

An identity with a lesser role SHALL be able to list grants and see their declarations, so that
it can tell what it may invoke and draft a proposal for a human.

#### Scenario: An agent cannot approve its own capability

- **WHEN** an identity without the admin role attempts to create a grant
- **THEN** the system refuses it as forbidden, and no grant is created

#### Scenario: An agent can see what exists

- **WHEN** an identity without the admin role lists grants
- **THEN** the system returns them, including the secrets each declares

### Requirement: An implementation is stored, never referenced

When a grant's implementation is a script, the system SHALL store the script's content.

The system SHALL NOT accept a grant whose implementation is a path, a URL, or any other
reference resolved at execution time.

#### Scenario: A script is ingested

- **WHEN** a grant is created with a script
- **THEN** the content is stored with the grant and is returned when the grant is shown

#### Scenario: What was approved is what runs

- **WHEN** a grant has been created and any file the author originally wrote it in is changed
- **THEN** the stored grant is unaffected

### Requirement: An implementation is exactly one of an adapter or a script

A grant SHALL name either a built-in adapter or a script, and SHALL NOT have both or neither.

The system SHALL reject a grant naming an adapter it does not recognise, rather than deferring
the failure to execution.

#### Scenario: Both is refused

- **WHEN** a grant is created naming both an adapter and a script
- **THEN** the system refuses it and names the conflict

#### Scenario: An unknown adapter is refused at creation

- **WHEN** a grant is created naming an adapter the system does not implement
- **THEN** the system refuses it at creation, when a human is present to see it

### Requirement: A grant is validated before it becomes runnable

The system SHALL verify at creation that every secret the grant declares exists, and SHALL
refuse the grant otherwise.

#### Scenario: A grant naming a missing secret is refused

- **WHEN** a grant is created declaring a secret that does not exist
- **THEN** the system refuses it and names the missing secret

### Requirement: A chain is linear, finite, and checked

A grant MAY name other grants to run after it succeeds, in order.

The system SHALL refuse a chain that names a grant which does not exist, and SHALL refuse a
chain that could revisit a grant already in it.

#### Scenario: A chain naming a missing grant is refused

- **WHEN** a grant is created chaining to a name no grant has
- **THEN** the system refuses it and names the missing grant

#### Scenario: A cycle is refused

- **WHEN** a grant would create a chain that returns to a grant already in that chain
- **THEN** the system refuses it and reports the cycle

#### Scenario: A chain is ordered

- **WHEN** a grant declaring a chain is shown
- **THEN** the chained grants appear in the order they were declared

### Requirement: Grant names are stable and unique

A grant SHALL be identified by a name unique on the server, and that name SHALL be what a caller
uses to refer to it.

#### Scenario: A duplicate name is refused

- **WHEN** a grant is created with a name already in use
- **THEN** the system refuses it rather than replacing the existing grant

