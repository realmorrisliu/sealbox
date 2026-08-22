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

