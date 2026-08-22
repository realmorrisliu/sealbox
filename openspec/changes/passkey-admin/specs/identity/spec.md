## MODIFIED Requirements

### Requirement: Credentials are stored only as hashes

The system SHALL store a bearer credential only as a hash, and SHALL return it exactly once, at
creation.

The system SHALL NOT provide any interface that returns an existing credential.

An admin identity SHALL have no bearer credential at all: it authenticates by proving possession
of a registered authenticator, and what the system stores about that authenticator SHALL NOT be
sufficient to authenticate as it.

#### Scenario: A credential is shown once

- **WHEN** an identity that uses a bearer credential is created
- **THEN** its credential is returned in that response and never again

#### Scenario: The database yields no usable credential

- **WHEN** the stored records for every identity are read directly
- **THEN** no value in them can be presented as a credential to authenticate

#### Scenario: An admin has nothing to leak

- **WHEN** an admin identity is created
- **THEN** no credential is returned, and none is stored
