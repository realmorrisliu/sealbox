## MODIFIED Requirements

### Requirement: Business endpoints require authentication

Every endpoint other than the health probes SHALL require authentication as a **named identity**,
and SHALL additionally require that the identity's role permits the action.

The system SHALL reject unauthenticated requests without disclosing whether the requested
resource exists, and SHALL distinguish *unauthenticated* from *authenticated but not permitted*,
so that a caller can tell a missing credential from an insufficient one.

The system SHALL NOT accept a shared or static credential not bound to an identity.

#### Scenario: An unauthenticated request is rejected

- **WHEN** a request to a business endpoint carries no credential or an invalid one
- **THEN** the system rejects it as unauthorised
- **AND** the response does not reveal whether the named secret, master key, or resource exists

#### Scenario: An authenticated request without the required role is rejected

- **WHEN** an identity makes a request its role does not permit
- **THEN** the system rejects it as forbidden, distinctly from unauthorised
- **AND** the attempt is recorded in the audit trail
