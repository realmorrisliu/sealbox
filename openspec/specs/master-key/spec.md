# master-key Specification

## Purpose
Master keys are the keypairs that secrets' data keys are encrypted under. This capability covers
their registration, which one is current, the distinction between keys the server holds and keys it
does not, and rekeying — re-encrypting existing data keys under a different master key. It exists
so that no operation anywhere in sealbox ever requires a client to hand over private key material.
## Requirements
### Requirement: Master key registration

The system SHALL accept registration of a master key by its public half only. A registered master
key SHALL be identified by a stable identifier that secrets reference.

The system SHALL NOT accept, store, or transmit the private half of a client-registered master key.

#### Scenario: Registering a public key

- **WHEN** a client registers a master key by submitting a public key
- **THEN** the system stores it, assigns it an identifier, and returns that identifier

#### Scenario: Private key material is refused

- **WHEN** a registration request includes private key material
- **THEN** the system rejects the request and stores nothing

### Requirement: Server-held and cold master keys

Each master key SHALL be recorded as either **server-held** — the system possesses its private half
— or **cold** — the system does not.

A secret encrypted under a server-held master key SHALL be decryptable by the system. A secret
encrypted under a cold master key SHALL NOT be decryptable by the system under any circumstances,
including rekey.

#### Scenario: A secret under a cold master key cannot be read by the server

- **WHEN** an operation would require decrypting a secret whose master key is cold
- **THEN** the system fails that operation and reports that the secret is cold

#### Scenario: New secrets use the current server-held key

- **WHEN** a secret is created and no master key is specified
- **THEN** its data key is encrypted under the current server-held master key

### Requirement: Rekey requires no client key material

Rekeying SHALL re-encrypt the data keys of secrets from one master key to another using key material
the system already holds.

The system SHALL NOT expose any interface that accepts a private key in order to perform a rekey.

#### Scenario: Rekeying secrets under a server-held key

- **WHEN** a rekey is requested from a server-held master key to another registered master key
- **THEN** the system re-encrypts the data keys of every secret referencing the source key, and each
  such secret afterwards references the destination key
- **AND** the plaintext values of those secrets are unchanged

#### Scenario: A request carrying a private key is rejected

- **WHEN** a rekey request includes private key material
- **THEN** the system rejects the request, performs no re-encryption, and does not log the submitted
  material

#### Scenario: Rekeying from a cold key is refused

- **WHEN** a rekey is requested whose source master key is cold
- **THEN** the system rejects the request and reports that the source key is not server-held

### Requirement: Rekey is atomic

A rekey SHALL either complete for every affected secret or leave every affected secret unchanged.

The system SHALL NOT leave secrets split across the source and destination master keys after a
failed rekey.

#### Scenario: A failure mid-rekey changes nothing

- **WHEN** re-encryption fails for any secret during a rekey
- **THEN** no secret's master key reference is changed, and every secret remains decryptable exactly
  as before the attempt

### Requirement: Rekey and rotate are distinct operations

The system SHALL use **rekey** exclusively for re-encrypting a data key under a different master key,
and SHALL NOT use the word *rotate* for it in any interface, audit record, or error message.

*Rotate* is reserved for replacing a secret's value.

#### Scenario: A rekey is recorded unambiguously

- **WHEN** a rekey completes
- **THEN** the record of it identifies the operation as a rekey and names the source and destination
  master keys
- **AND** it is distinguishable from a record of a secret's value changing

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

