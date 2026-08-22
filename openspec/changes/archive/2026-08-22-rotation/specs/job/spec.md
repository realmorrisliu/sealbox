## ADDED Requirements

### Requirement: A rotation job carries a generated value

When a job is part of a rotation, the claim SHALL provide the newly generated value to the
implementation alongside the grant's declared secrets.

The value SHALL be provided the same way declared secrets are, and SHALL NOT be distinguishable
to the implementation from any other injected value.

#### Scenario: The implementation receives the new value

- **WHEN** a runner claims a rotation job
- **THEN** the claim carries the new value in addition to the grant's declared secrets

#### Scenario: A non-rotation job carries no such value

- **WHEN** a runner claims an ordinary job
- **THEN** no generated value is present

### Requirement: A captured value is returned outside the job record

When a job is part of a capturing rotation, the runner SHALL report the captured value
separately from the job's output.

The system SHALL NOT store a captured value in the job record, and SHALL NOT return it in any
response.

#### Scenario: Capture is separate from output

- **WHEN** a runner reports a captured value along with output
- **THEN** the output is stored and readable, and the captured value is not stored with it

#### Scenario: The job record stays free of it

- **WHEN** a job that captured a value is read afterwards
- **THEN** nothing in it contains the captured value
