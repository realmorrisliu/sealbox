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
current one, whether the value was supplied or generated.

#### Scenario: A second write adds a version

- **WHEN** a value is stored under a key that already exists
- **THEN** a new version is created and the previous version remains retrievable by number

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

