## ADDED Requirements

### Requirement: A value is replaced only by a successful rotation

Replacing a secret's value SHALL require a grant, and the new value SHALL become current only if
that grant succeeds.

While a rotation is in progress the new value SHALL be stored but SHALL NOT be readable, listed,
or provided to any grant.

If the grant fails, the system SHALL discard the new value and leave the previous version current
and unchanged.

#### Scenario: A successful rotation

- **WHEN** a rotation's grant exits successfully
- **THEN** the new value becomes the secret's current version

#### Scenario: A failed rotation changes nothing

- **WHEN** a rotation's grant fails
- **THEN** the secret's current value is the one it had before, unchanged
- **AND** the new value is not retained

#### Scenario: A pending value is invisible

- **WHEN** a rotation is in progress and the secret is read or listed
- **THEN** the previous value is what is returned, and the pending one is not mentioned

### Requirement: The system generates the value, not the implementation

The system SHALL generate the new value and provide it to the grant.

The system SHALL NOT accept a new value supplied by the caller requesting the rotation.

#### Scenario: The implementation receives a value it did not create

- **WHEN** a rotation runs
- **THEN** the implementation is given the new value by the system

#### Scenario: A caller cannot choose the new value

- **WHEN** a rotation request includes a value
- **THEN** the system rejects the request rather than using it

### Requirement: A composed value may be captured from the grant

A rotation MAY be configured to store what the grant produced instead of the generated value, for
values that are composed or issued upstream rather than raw randomness.

A captured value SHALL be stored enveloped and SHALL NOT appear in the job record, in logs, or in
any response.

#### Scenario: Capturing a composed value

- **WHEN** a rotation is run in capture mode and the grant emits a value
- **THEN** that value becomes the secret's new current version

#### Scenario: A captured value does not leak through the job

- **WHEN** a rotation captures a value
- **THEN** the job record, its output, and every response contain no part of it

#### Scenario: Capturing nothing fails the rotation

- **WHEN** a rotation is run in capture mode and the grant emits no value
- **THEN** the rotation fails and the previous version remains current

## MODIFIED Requirements

### Requirement: Writing a secret creates a new version

Storing a value under an existing key SHALL create a new version rather than replacing the
current one, whether the value was supplied, generated, or produced by a rotation.

A version created by a rotation that did not succeed SHALL NOT be retained, and SHALL NOT
consume a version number that a later write would then skip in a way that suggests a value is
missing.

#### Scenario: A second write adds a version

- **WHEN** a value is stored under a key that already exists
- **THEN** a new version is created and the previous version remains retrievable by number

#### Scenario: A failed rotation leaves no gap

- **WHEN** a rotation fails and a later write succeeds
- **THEN** the version numbering does not imply that a version exists which does not
