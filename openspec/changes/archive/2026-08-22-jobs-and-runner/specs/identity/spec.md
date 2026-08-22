## MODIFIED Requirements

### Requirement: Roles determine what an identity may do

The system SHALL support the roles **admin**, **operator**, **agent**, and **runner**.

The first three are ordered, each admitting everything the one below it may do:

- an **agent** may invoke approved capabilities and read metadata, and may not create secrets,
  manage identities, or approve capabilities;
- an **operator** may additionally store secrets;
- an **admin** may additionally manage identities and approve capabilities.

**runner** is not part of that order. Its permissions are *disjoint*: it may claim jobs addressed
to it and report their results, and it may do nothing else — it cannot invoke a grant, read a
secret by name, list secrets, or read the audit trail. Equally, no other role may claim a job.

The system SHALL evaluate authorisation in one place rather than per endpoint, so that an
endpoint added later is refused by default rather than exposed by omission.

#### Scenario: An agent cannot widen its own authority

- **WHEN** an identity with the agent role attempts to create an identity or approve a capability
- **THEN** the system refuses the request as forbidden, distinctly from unauthenticated

#### Scenario: An agent may invoke

- **WHEN** an identity with the agent role invokes an already-approved capability
- **THEN** the system permits it

#### Scenario: A runner may only take what it is given

- **WHEN** an identity with the runner role attempts to invoke a grant, list secrets, or read the
  audit trail
- **THEN** the system refuses it as forbidden

#### Scenario: Being an admin does not confer the runner's permission

- **WHEN** an identity with the admin role attempts to claim a job
- **THEN** the system refuses it, because runner permissions are disjoint rather than beneath
  admin's

#### Scenario: A new endpoint is not exposed by default

- **WHEN** an endpoint is added without declaring a required role
- **THEN** requests to it are refused rather than permitted
