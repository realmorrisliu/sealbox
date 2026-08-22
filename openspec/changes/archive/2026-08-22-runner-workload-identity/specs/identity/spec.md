## ADDED Requirements

### Requirement: A runner may authenticate as a workload rather than with a stored credential

The system SHALL permit a runner identity to be bound to a registered token issuer and an exact
subject, and SHALL then accept a signed token from that issuer as that identity's authentication.

The system SHALL verify the signature against the issuer's registered keys, reject an expired or
not-yet-valid token, require the expected audience, and require the subject to match exactly.

The system SHALL NOT require any network access to the issuer in order to verify a token.

#### Scenario: A workload token authenticates a runner

- **WHEN** a runner presents a valid, unexpired token from its bound issuer with the expected
  audience and exact subject
- **THEN** the system authenticates it as that identity

#### Scenario: A token from another subject is refused

- **WHEN** a token from the bound issuer carries a different subject
- **THEN** the system refuses it, even if the subject differs only by a suffix

#### Scenario: A token minted for another audience is refused

- **WHEN** a token carries an audience other than the one expected
- **THEN** the system refuses it

#### Scenario: An expired token is refused

- **WHEN** a token's expiry has passed
- **THEN** the system refuses it

#### Scenario: A token signed by an unregistered key is refused

- **WHEN** a token's signature does not verify against any key registered for its issuer
- **THEN** the system refuses it

### Requirement: Revocation ends workload authentication too

Revoking an identity SHALL end its access regardless of what its issuer continues to sign.

#### Scenario: A revoked runner is refused a valid token

- **WHEN** a revoked identity presents an otherwise valid workload token
- **THEN** the system refuses it

### Requirement: Registering an issuer is an admin operation

Registering, updating, or removing a token issuer SHALL require an admin, authenticated as admin
operations are.

An issuer MAY hold more than one key at once, so that a rotation can register the new key before
the old one is removed.

#### Scenario: Only an admin registers an issuer

- **WHEN** an identity that is not an admin attempts to register an issuer
- **THEN** the system refuses it

#### Scenario: A key rotation does not interrupt authentication

- **WHEN** an issuer holds both an old and a new key and a runner presents a token signed by either
- **THEN** the system authenticates it
