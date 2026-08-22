# secret Specification

## Purpose
How a secret's value comes into existence, how it is versioned and expired, and what may be
learned about it without being allowed to read it. It exists because the safest value is one
that has been in as few places as possible — ideally only inside sealbox and inside whatever
consumes it.
## Requirements
### Requirement: A supplied value never travels on a command line

When a value is supplied by a caller, the system SHALL accept it from a request body or from
standard input.

The system SHALL NOT provide any interface that accepts a secret's value as a command-line
argument.

#### Scenario: Storing a supplied value

- **WHEN** a caller supplies a value on standard input
- **THEN** the system stores it, enveloped, as a new version

#### Scenario: No argument form exists

- **WHEN** the available commands are examined
- **THEN** none of them takes a secret's value as a positional or flag argument

### Requirement: The server can generate a value

The system SHALL be able to generate a secret's value itself, from a cryptographically secure
random source, and store it without the plaintext leaving the server.

A generated value SHALL NOT be returned to the caller, in the response that created it or any
other.

#### Scenario: Generating a value

- **WHEN** a caller requests generation of a secret
- **THEN** the system generates a value, stores it enveloped, and reports only that the secret
  now exists at a given version

#### Scenario: The generated value is not disclosed

- **WHEN** a secret is generated
- **THEN** no part of the response contains the value

#### Scenario: Two generated secrets differ

- **WHEN** two secrets are generated with identical parameters
- **THEN** their values differ

### Requirement: Generation parameters are explicit and bounded

The system SHALL support generating at least a printable password and a hexadecimal string, with
a caller-specified length.

The system SHALL reject a length that would produce a value with less entropy than a stated
minimum, rather than silently producing a weak secret.

#### Scenario: An unreasonably short length is refused

- **WHEN** generation is requested with a length below the minimum
- **THEN** the system refuses the request and names the minimum

#### Scenario: A default length is applied

- **WHEN** generation is requested without a length
- **THEN** the system uses a documented default that meets the minimum

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

### Requirement: Listing reveals metadata, never values

Listing secrets SHALL return the key, the current version, timestamps, and expiry, and SHALL NOT
return any value or ciphertext.

Listing SHALL omit expired secrets.

#### Scenario: A listing carries no values

- **WHEN** secrets are listed
- **THEN** each entry has a key, a version, and timestamps
- **AND** no entry contains a value, an encrypted value, or an encrypted data key

#### Scenario: Expired secrets are not listed

- **WHEN** a secret's expiry has passed and secrets are listed
- **THEN** that secret does not appear

### Requirement: A client reports listing accurately

A client SHALL NOT report that an operation is unsupported when the server supports it.

#### Scenario: Listing works through the client

- **WHEN** a caller lists secrets through the client and the server supports listing
- **THEN** the client returns the server's result rather than a message claiming it cannot

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

