## MODIFIED Requirements

### Requirement: Creating a grant requires an approval, not a role

Any authenticated identity SHALL be able to submit a grant for approval. Submitting SHALL create
nothing.

The system SHALL create the grant only when a human has approved it by signing for it, and SHALL
permit only an identity with the admin role to remove one.

An identity with a lesser role SHALL be able to list grants and see their declarations, so that it
can tell what it may invoke and draft a proposal for a human.

#### Scenario: An agent drafts, and nothing exists yet

- **WHEN** an identity without the admin role submits a grant
- **THEN** the system stages it for approval and no grant exists

#### Scenario: An agent cannot approve its own capability

- **WHEN** an identity attempts to approve a staged grant without a signature from a registered
  admin authenticator
- **THEN** the system refuses it, and no grant is created

#### Scenario: An agent cannot remove one either

- **WHEN** an identity without the admin role attempts to remove a grant
- **THEN** the system refuses it as forbidden

#### Scenario: An agent can see what exists

- **WHEN** an identity without the admin role lists grants
- **THEN** the system returns them, including the secrets each declares
