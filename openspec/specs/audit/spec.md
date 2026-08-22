# audit Specification

## Purpose
The record of what was attempted, by whom, and whether it was allowed. It exists because letting
an agent act is only acceptable if the question "which agent did this, and when" can be answered
afterwards — and because a refused attempt is the signal that something has gone wrong.
## Requirements
### Requirement: Every attempt is recorded

The system SHALL record every authenticated attempt against a business endpoint, whether it
succeeded or was refused, with the identity, the time, the action, the resource it named, and the
outcome.

The system SHALL record refusals, including attempts by an identity whose role does not permit
the action.

#### Scenario: A successful action is recorded

- **WHEN** an identity performs a permitted action
- **THEN** an audit record exists naming that identity, the action, the resource, and success

#### Scenario: A refused action is recorded

- **WHEN** an identity attempts an action its role does not permit
- **THEN** an audit record exists naming that identity, the action, and the refusal

#### Scenario: An unauthenticated attempt is recorded without inventing an identity

- **WHEN** a request presents no valid credential
- **THEN** the attempt is recorded without attributing it to any identity

### Requirement: Audit records never contain secret values

An audit record SHALL NOT contain a secret's plaintext, a credential, or key material, in any
field, including error details.

#### Scenario: A failure involving a secret

- **WHEN** an action fails while handling a secret
- **THEN** the audit record identifies the secret by name and contains no part of its value

### Requirement: Audit records are append-only

The system SHALL NOT provide any interface that modifies or deletes an audit record.

#### Scenario: No interface alters history

- **WHEN** the full set of available operations is examined
- **THEN** none of them updates or removes an existing audit record

### Requirement: The trail is readable by identity, action, and time

The system SHALL allow audit records to be read filtered by identity, by action, and by time
range, most recent first.

Reading the audit trail SHALL be permitted to every authenticated identity, since concealing it
from an agent protects nothing the agent could not already observe.

#### Scenario: Narrowing to one identity

- **WHEN** the trail is read filtered by an identity
- **THEN** only that identity's records are returned, most recent first

#### Scenario: Narrowing to a time range

- **WHEN** the trail is read filtered by a time range
- **THEN** only records within it are returned

