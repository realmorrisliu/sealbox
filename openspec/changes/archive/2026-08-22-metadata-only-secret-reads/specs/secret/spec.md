## ADDED Requirements

### Requirement: Reading a secret returns metadata, never ciphertext

Reading a secret SHALL return the key, the version, the master key it is encrypted under, the
timestamps, and the expiry.

The system SHALL NOT return a secret's value, its encrypted value, or its encrypted data key
through any interface, at any role, under any parameter.

#### Scenario: A read carries no ciphertext

- **WHEN** a secret is read
- **THEN** the response has a key, a version, and timestamps
- **AND** it contains no value, no encrypted value, and no encrypted data key

#### Scenario: No parameter reveals it

- **WHEN** a caller asks for a secret's ciphertext by any parameter
- **THEN** the system returns metadata as it would otherwise, revealing nothing further

### Requirement: A cold secret is read without the server

A secret encrypted under a master key the system does not hold SHALL be readable by a tool
operating directly on a database file and that key, with no running server involved.

That tool SHALL refuse a secret whose master key differs from the key supplied.

#### Scenario: The server is gone and the secret is still readable

- **WHEN** the holder of a cold master key has a copy of the database and no running server
- **THEN** they can read that secret

#### Scenario: The wrong key is named as such

- **WHEN** a key is supplied that did not encrypt the secret
- **THEN** the tool says so rather than failing inside a decryption

### Requirement: A client offers no command that does nothing

The client SHALL NOT provide a command whose implementation reports that the operation is
unsupported.

#### Scenario: An unimplemented command is absent, not inert

- **WHEN** a caller looks for an operation the system does not support
- **THEN** no command for it exists, rather than one that accepts the call and does nothing
