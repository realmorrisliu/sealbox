# http-api Specification

## Purpose
The transport-level contract of the sealbox server: which API versions exist, which endpoints
require authentication, and what the API deliberately does not support. It exists to keep the
server's exposed surface exactly as large as its clients need and no larger — sealbox holds every
credential in a system, so surface it does not need is surface it should not have.
## Requirements
### Requirement: Only version v1 exists

The API SHALL expose exactly one version, `v1`.

The system SHALL NOT enumerate, advertise, or reserve version identifiers it does not serve.

#### Scenario: A v1 request is served

- **WHEN** a request is made to a `v1` endpoint
- **THEN** the system handles it

#### Scenario: An unknown version is rejected

- **WHEN** a request is made with any version identifier other than `v1`
- **THEN** the system rejects it as an unsupported version, with no distinction between versions that
  were once planned and versions that never existed

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

### Requirement: Health probes require no authentication

The system SHALL expose a liveness probe and a readiness probe that require no credential. The
readiness probe SHALL verify that the data store is reachable.

Health probe responses SHALL NOT disclose any information about stored data, including counts.

#### Scenario: Readiness reflects data store availability

- **WHEN** the readiness probe is called and the data store cannot be reached
- **THEN** the system reports itself not ready

#### Scenario: Probes leak nothing

- **WHEN** either probe is called without a credential
- **THEN** the response contains only liveness or readiness status

### Requirement: No cross-origin access

The API SHALL NOT emit CORS response headers, and SHALL NOT provide configuration to enable them.

The system SHALL NOT vary this behavior by build profile.

#### Scenario: No CORS headers are returned

- **WHEN** any request is made, with or without an `Origin` header
- **THEN** the response contains no `Access-Control-Allow-*` headers

#### Scenario: A preflight request is not honoured

- **WHEN** an `OPTIONS` preflight request is made
- **THEN** the system does not respond in a way that authorises a cross-origin request

#### Scenario: Debug builds behave identically

- **WHEN** the server is built with debug assertions enabled
- **THEN** its cross-origin behavior is identical to a release build

### Requirement: Requests are traceable

Every request SHALL be assigned or given a request identifier that is propagated to the response and
included in log records for that request.

#### Scenario: A client-supplied identifier is preserved

- **WHEN** a request carries a request identifier
- **THEN** the same identifier appears in the response and in the log records for that request

#### Scenario: A missing identifier is generated

- **WHEN** a request carries no request identifier
- **THEN** the system generates one and returns it

