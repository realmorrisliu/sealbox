## Purpose

The envelope encryption contract for stored secrets: what a stored secret consists of, and the
guarantee that data written by one build stays readable by later ones. It exists because nothing
previously stated this, which let a dependency upgrade silently threaten the readability of every
stored credential.

## ADDED Requirements

### Requirement: Secrets are stored under envelope encryption

Every stored secret SHALL be encrypted with a data key generated for that secret, and that data key
SHALL itself be stored encrypted under a master key.

The system SHALL NOT store a secret's value in plaintext, and SHALL NOT store a data key in
plaintext.

#### Scenario: Storing a secret

- **WHEN** a secret value is stored
- **THEN** the stored record contains the encrypted value, the encrypted data key, and a reference
  to the master key the data key was encrypted under
- **AND** neither the value nor the data key is recoverable from the record without the master key's
  private half

#### Scenario: Each secret has its own data key

- **WHEN** two secrets are stored
- **THEN** they are encrypted under different data keys

### Requirement: Stored data survives upgrades

A secret written by any build SHALL remain decryptable by every later build, without a data
migration.

A change to a dependency version, a library API, or an internal type SHALL NOT alter the stored
format or the parameters of the cryptographic construction.

#### Scenario: An older record is read by a newer build

- **WHEN** a build reads a secret written by an earlier build
- **THEN** it decrypts to exactly the original value

#### Scenario: A dependency upgrade preserves readability

- **WHEN** a cryptography dependency is upgraded
- **THEN** records written before the upgrade decrypt correctly afterwards
- **AND** if that cannot be guaranteed, the upgrade is not applied without an accompanying,
  explicitly designed migration

### Requirement: Decryption failure is never silent

The system SHALL report a decryption failure as an error identifying which secret failed.

The system SHALL NOT return a partial value, an empty value, or a placeholder when decryption fails.

#### Scenario: Corrupt or unreadable ciphertext

- **WHEN** a stored secret cannot be decrypted
- **THEN** the operation fails with an error naming the secret
- **AND** no value is returned to the caller

### Requirement: Key material never appears in diagnostics

Plaintext secret values, data keys, and private master keys SHALL NOT appear in logs, error
messages, traces, or any response body.

#### Scenario: An error during a cryptographic operation

- **WHEN** an encryption or decryption operation fails
- **THEN** the resulting error and any log record it produces contain no key material and no
  plaintext value
