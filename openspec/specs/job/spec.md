# job Specification

## Purpose
One requested execution of a grant: its parameters, the runner it is addressed to, and its
result. It exists because a grant has to run *somewhere*, and the only acceptable somewhere is
neither the hosted server nor the agent's own machine.
## Requirements
### Requirement: A job is a request to execute one grant

Submitting a job SHALL name an existing grant and MAY supply parameters. The system SHALL refuse
a job naming a grant that does not exist.

A caller SHALL NOT be able to supply a command, a script, or a secret name as part of a job.

#### Scenario: Submitting a job

- **WHEN** an identity permitted to invoke submits a job naming an approved grant
- **THEN** the job is queued for the runner that grant declares

#### Scenario: A job naming no grant is refused

- **WHEN** a job names a grant that does not exist
- **THEN** the system refuses it and nothing is queued

#### Scenario: A caller cannot smuggle in an implementation

- **WHEN** a job submission includes a field describing what to execute
- **THEN** the system rejects the submission rather than ignoring the field

### Requirement: Only the addressed runner may claim a job

A job SHALL be claimable only by an identity with the runner role whose name matches the runner
the grant declares.

The system SHALL give a claimed job to exactly one runner.

#### Scenario: A runner claims work addressed to it

- **WHEN** a runner polls and a job exists for it
- **THEN** the system returns that job and marks it claimed

#### Scenario: A runner sees nothing addressed elsewhere

- **WHEN** a runner polls and the only pending jobs name a different runner
- **THEN** the system returns nothing to it

#### Scenario: No other role can claim

- **WHEN** an identity that is not a runner attempts to claim a job
- **THEN** the system refuses it as forbidden

#### Scenario: A job is claimed once

- **WHEN** two claims arrive for the same pending job
- **THEN** exactly one of them receives it

### Requirement: A claim carries exactly the declared secrets

When a job is claimed, the system SHALL provide the grant's implementation and the plaintext of
**only** the secrets that grant declares.

The system SHALL NOT provide any secret the grant does not declare, and SHALL NOT allow a runner
to request a secret by name.

#### Scenario: Undeclared secrets are absent

- **WHEN** a runner claims a job whose grant declares one secret
- **THEN** it receives that secret's value and no other

#### Scenario: A runner cannot ask

- **WHEN** the operations available to a runner are examined
- **THEN** none of them retrieves a secret by name

### Requirement: A result is reported by the runner, not the caller

The system SHALL accept a result only from the runner that claimed the job.

The submitting caller SHALL receive the exit status and output, and SHALL NOT receive any secret
value.

#### Scenario: Reporting a result

- **WHEN** the claiming runner reports an outcome
- **THEN** the job records it and the waiting caller receives it

#### Scenario: Another identity cannot report

- **WHEN** an identity other than the claiming runner reports a result for that job
- **THEN** the system refuses it and the job is unchanged

### Requirement: An abandoned job fails rather than waiting forever

A job claimed but not reported within a bounded period SHALL be marked failed, and SHALL NOT be
handed to another runner.

#### Scenario: A runner dies mid-job

- **WHEN** a claimed job goes unreported past the timeout
- **THEN** it is marked failed with a reason saying so
- **AND** it is not retried

#### Scenario: Nothing is retried automatically

- **WHEN** any job fails, for any reason
- **THEN** the system does not execute it again on its own

### Requirement: Secrets are made available in the form the consumer needs

The system SHALL support providing a secret to an implementation as an environment variable, as
the path to a file containing it, and as the path to a file rendering several secrets in
`KEY=value` form.

A file created for this purpose SHALL be readable only by the executing user and SHALL be removed
when execution ends, including on failure.

#### Scenario: A file-shaped credential

- **WHEN** a grant declares a secret to be provided as a file
- **THEN** the implementation receives a path, and the file at it contains exactly the value

#### Scenario: Several secrets at once

- **WHEN** a grant declares secrets to be provided as an env-file
- **THEN** the implementation receives a path to a file containing each as `KEY=value`

#### Scenario: Files do not outlive the job

- **WHEN** execution ends, whether it succeeded or failed
- **THEN** no file created to carry a secret remains

### Requirement: Parameters are arguments, never commands

Parameters supplied with a job SHALL be substituted into the implementation's argument vector and
SHALL NOT be interpreted by a shell.

#### Scenario: A parameter that looks like a command

- **WHEN** a job supplies a parameter whose value contains shell metacharacters
- **THEN** the implementation receives it as a single literal argument
- **AND** nothing in it is executed

### Requirement: A chain runs in order and stops at the first failure

When a grant declares a chain, the system SHALL run each grant in the declared order after the
previous one succeeds, and SHALL stop at the first failure.

The system SHALL record which step failed.

#### Scenario: A chain completes

- **WHEN** a grant with a chain succeeds and each chained grant succeeds
- **THEN** all of them ran, in the declared order

#### Scenario: A chain stops

- **WHEN** a step in a chain fails
- **THEN** no later step runs, and the failure names the step

