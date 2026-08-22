## ADDED Requirements

### Requirement: A first boot may generate the server's master key

The system SHALL generate a server master key at the configured path when, and only when, exactly
one path is configured, no file exists at it, and the store holds no master key and no secret.

The system SHALL write it readable only by its owner, and SHALL log a fingerprint rather than the
key.

In every other case, an unreadable master key path SHALL remain fatal.

#### Scenario: A fresh server brings itself up

- **WHEN** a server starts with an empty store and a configured path holding no file
- **THEN** it generates a master key there and starts

#### Scenario: A restored database does not get a new key

- **WHEN** a server starts with a store holding secrets and no file at the configured path
- **THEN** it refuses to start, rather than generating a key under which nothing can be read

#### Scenario: A mistyped path on an existing deployment is refused

- **WHEN** a server with existing secrets is pointed at a path that does not exist
- **THEN** it refuses to start and names the path

#### Scenario: The key is never disclosed by starting

- **WHEN** a master key is generated at first boot
- **THEN** nothing in the logs contains the key
