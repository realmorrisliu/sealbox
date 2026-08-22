# admin-approval Specification

## Purpose
How a human proves they are an admin, and what makes an approval worth trusting. It exists
because a credential in a file can be read by an agent on the same machine, and because a
terminal cannot be a trusted display — its output is written by whatever process is running.
## Requirements
### Requirement: No admin credential exists at rest

An admin identity SHALL authenticate by proving possession of a registered authenticator, and
SHALL NOT have a credential that can be stored, copied, or presented on its own.

The system SHALL NOT store anything that could be replayed to authenticate as an admin.

#### Scenario: Nothing on the filesystem authenticates as admin

- **WHEN** every file an admin's machine holds is read
- **THEN** none of them can be presented to the system to act as that admin

#### Scenario: Stored registration data cannot be replayed

- **WHEN** everything the system stores about a registered authenticator is read
- **THEN** none of it can be used to produce a valid authentication

### Requirement: Authentication is a challenge the server issues

The system SHALL issue a single-use challenge, accept a signature over it, and reject a challenge
that has been used, has expired, or was not issued by it.

#### Scenario: A signature over a fresh challenge is accepted

- **WHEN** a registered authenticator signs a challenge the system issued
- **THEN** the system accepts it

#### Scenario: A replayed challenge is refused

- **WHEN** a signature over a challenge already used is presented again
- **THEN** the system refuses it

#### Scenario: An expired challenge is refused

- **WHEN** a signature arrives after its challenge has expired
- **THEN** the system refuses it

### Requirement: What is approved is rendered by the server

An approval SHALL be presented on a page the system renders, and the thing signed SHALL be the
thing presented.

The system SHALL NOT accept an approval whose subject differs from what was shown.

#### Scenario: The declaration shown is the declaration approved

- **WHEN** a human approves a pending grant
- **THEN** what they signed is the declaration the system rendered, not one supplied by the caller

#### Scenario: A caller cannot substitute the subject

- **WHEN** an approval is submitted for a subject other than the one the challenge was issued for
- **THEN** the system refuses it

### Requirement: A session is short, in memory, and not a credential at rest

Authenticating MAY yield a session usable for further admin operations for a bounded period.

A session SHALL NOT be written to disk by the system, and SHALL expire.

#### Scenario: A session permits further operations

- **WHEN** an admin authenticates and then performs several operations within the period
- **THEN** each is permitted without authenticating again

#### Scenario: A session expires

- **WHEN** an operation is attempted after the session's period has elapsed
- **THEN** the system refuses it and a new authentication is required

#### Scenario: Sessions do not survive a restart

- **WHEN** the system restarts
- **THEN** every session issued before it is no longer accepted

### Requirement: Admin operations require a session, not a token

The system SHALL refuse an admin operation presented with a bearer token, and SHALL require a
session obtained by authenticating with a registered authenticator.

#### Scenario: A bearer token does not perform admin work

- **WHEN** an admin operation is attempted with any bearer token
- **THEN** the system refuses it as unauthorised, whatever the token belongs to

#### Scenario: Other roles are unaffected

- **WHEN** an agent, operator, or runner presents its bearer token
- **THEN** it is authenticated as before

### Requirement: The first admin registers through a bounded enrolment

The system SHALL provide a way for an identity with no registered authenticator to register one,
using a single-use link that expires.

#### Scenario: Enrolling

- **WHEN** an admin identity with no authenticator follows a valid enrolment link
- **THEN** it may register one, and the link cannot be used again

#### Scenario: An expired link is refused

- **WHEN** an enrolment link is followed after it has expired
- **THEN** the system refuses it and no authenticator is registered

#### Scenario: Enrolment cannot replace an existing authenticator

- **WHEN** an enrolment link is used for an identity that already has one
- **THEN** the system refuses it, so that a leaked link cannot displace a working credential

