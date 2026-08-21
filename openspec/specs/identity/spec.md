# identity Specification

## Purpose
Who a caller is, what they are allowed to do, how they prove it, and how that access is
withdrawn. It exists so that an agent's authority can be strictly narrower than a human's, and so
that "which agent did this" has an answer.
## Requirements
### Requirement: Every caller is a named identity

The system SHALL identify every authenticated caller as a named identity holding exactly one
role.

The system SHALL NOT accept a shared credential, a default identity, or any authentication path
that does not resolve to a named identity.

#### Scenario: A request authenticates as an identity

- **WHEN** a request presents a valid credential
- **THEN** the system resolves it to exactly one identity and that identity's role

#### Scenario: An unrecognised credential is refused

- **WHEN** a request presents a credential matching no identity
- **THEN** the system refuses the request as unauthorised
- **AND** the response does not reveal whether the requested resource exists

### Requirement: Roles determine what an identity may do

The system SHALL support the roles **admin**, **operator**, and **agent**, and SHALL enforce
that:

- an **agent** may invoke approved capabilities and read metadata, and may not create secrets,
  manage identities, or approve capabilities;
- an **operator** may additionally store secrets;
- an **admin** may additionally manage identities and approve capabilities.

The system SHALL evaluate authorisation in one place rather than per endpoint, so that an
endpoint added later is refused by default rather than exposed by omission.

#### Scenario: An agent cannot widen its own authority

- **WHEN** an identity with the agent role attempts to create an identity or approve a capability
- **THEN** the system refuses the request as forbidden, distinctly from unauthenticated

#### Scenario: An agent may invoke

- **WHEN** an identity with the agent role invokes an already-approved capability
- **THEN** the system permits it

#### Scenario: A new endpoint is not exposed by default

- **WHEN** an endpoint is added without declaring a required role
- **THEN** requests to it are refused rather than permitted

### Requirement: Credentials are stored only as hashes

The system SHALL store an identity's credential only as a hash, and SHALL return the credential
exactly once, at creation.

The system SHALL NOT provide any interface that returns an existing credential.

#### Scenario: A credential is shown once

- **WHEN** an identity is created
- **THEN** its credential is returned in that response and never again

#### Scenario: The database yields no usable credential

- **WHEN** the stored records for every identity are read directly
- **THEN** no value in them can be presented as a credential to authenticate

### Requirement: Revocation is immediate and isolated

The system SHALL refuse every subsequent request from a revoked identity, and SHALL leave all
other identities working.

Revocation SHALL be recorded rather than deleted, so that audit records referring to the identity
remain meaningful.

#### Scenario: A revoked identity stops working at once

- **WHEN** an identity is revoked and then presents its credential
- **THEN** the system refuses the request

#### Scenario: Other identities are unaffected

- **WHEN** one identity is revoked
- **THEN** every other identity continues to authenticate with the credential it already had

#### Scenario: History survives revocation

- **WHEN** audit records exist for an identity that is later revoked
- **THEN** those records still identify which identity performed each action

### Requirement: The first identity is created through a bounded bootstrap

The system SHALL provide a way to create the first admin identity when no identity exists, using
a credential supplied at deployment.

That credential SHALL be accepted only while no identity exists, SHALL be usable at most once,
and SHALL be refused after a bounded period following server start.

The system SHALL NOT write that credential to logs, responses, or storage.

#### Scenario: Claiming a fresh server

- **WHEN** the bootstrap credential is presented and no identity exists
- **THEN** an admin identity is created and its credential returned once
- **AND** the act is recorded in the audit trail

#### Scenario: Bootstrap cannot be replayed

- **WHEN** the bootstrap credential is presented and any identity already exists
- **THEN** the system refuses it, regardless of the elapsed time

#### Scenario: The window closes

- **WHEN** the bootstrap credential is presented after the bounded period has elapsed
- **THEN** the system refuses it, even with no identity present

