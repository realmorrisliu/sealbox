## ADDED Requirements

### Requirement: An agent integrates through a skill and the CLI

The project SHALL provide a skill file describing how an agent uses sealbox, and SHALL NOT provide
an MCP server.

The skill SHALL reference the CLI reference for command detail rather than restating it.

#### Scenario: An agent has what it needs to act

- **WHEN** an agent that has never used sealbox reads the skill
- **THEN** it can store a secret, draft a grant, run one, and rotate a value without further
  instruction

### Requirement: The skill states what an agent must never do

The skill SHALL state that no command returns a secret's value, that a new credential is generated
by the server rather than invented by the agent, and that a secret name is never parameterised.

#### Scenario: A refusal is understood rather than worked around

- **WHEN** an agent is refused for one of those reasons
- **THEN** the skill explains why the refusal is the system working, not an obstacle to route
  around

### Requirement: The skill hands approval to a human

The skill SHALL state that a grant is submitted by the agent and approved by a human, and that the
agent's part ends at submission.

#### Scenario: An agent stops at the boundary

- **WHEN** an agent has drafted and submitted a grant
- **THEN** it reports the approval URL and waits, rather than retrying or finding another route

### Requirement: Worked examples are the template library

The project SHALL keep worked grant examples that the skill points to, and SHALL NOT provide a
scaffolding command or template flag.

#### Scenario: A new grant is written by imitation

- **WHEN** an agent is asked for a grant of a kind it has not written before
- **THEN** it reads the examples and writes one by imitation
