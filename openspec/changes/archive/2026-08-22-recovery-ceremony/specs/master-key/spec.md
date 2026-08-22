## ADDED Requirements

### Requirement: The server master key can be recovered without the server

The system SHALL support registering a recovery public key, and SHALL store its master key
encrypted under it as a recovery blob.

The system SHALL NOT store the corresponding private key, and SHALL NOT return the master key in
any other form.

A recovery blob SHALL be decryptable without a running system, given the recovery private key.

#### Scenario: A blob restores the master key

- **WHEN** a recovery blob and its recovery private key are supplied to the restore tool
- **THEN** the original master key is produced, with no server involved

#### Scenario: The blob reveals nothing on its own

- **WHEN** a recovery blob is obtained without the recovery private key
- **THEN** it does not yield the master key

#### Scenario: The private half is never uploaded

- **WHEN** recovery is initialised
- **THEN** only the public half reaches the system, and nothing stores the private half

### Requirement: A recovery blob is kept current automatically

When the server's master key changes, the system SHALL re-make the recovery blob for every
registered recovery key.

#### Scenario: A new master key refreshes the backup

- **WHEN** the server's master key changes and a recovery key is registered
- **THEN** the stored blob decrypts to the new master key

### Requirement: Initialisation is not complete until the backup is verified

The client SHALL verify a newly created recovery key by decrypting the stored blob with it, and
SHALL NOT report success until that succeeds.

#### Scenario: A backup that does not work is not reported as one

- **WHEN** the recovery key written to disk cannot decrypt the blob the system stored
- **THEN** the client reports failure rather than success

### Requirement: More than one recovery key may be registered

The system SHALL permit several recovery keys at once, each with its own blob, and registering one
SHALL NOT invalidate another.

#### Scenario: A second key is added without disturbing the first

- **WHEN** a second recovery key is registered
- **THEN** both keys can each recover the master key
