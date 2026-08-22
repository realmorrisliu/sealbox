# Agents invoke named grants, never compose commands

An agent asks sealbox to run a grant by name (`sealbox run deploy`). It cannot pass a command
line to be executed with a credential attached. Grants are declared by a human in a config file
that binds a command to the secrets it needs.

The reason is the threat model that distinguishes agents from ordinary clients: prompt injection.
Withholding plaintext is not sufficient on its own — an API of the shape
`run_with_secrets(secrets, command)` still lets a compromised agent do anything the credential
permits, including piping data to an attacker. Withholding plaintext *and* withholding command
composition reduces the attack surface to the set of commands a human already wrote down.

## Considered Options

Free-form commands with per-invocation human confirmation were rejected: confirmation fatigue
makes the check worthless in practice. Free-form commands with after-the-fact audit were rejected
because knowing that data was exfiltrated has little value once it has been.

## Consequences

Agents cannot improvise. Every new capability requires a human to add a grant first. This is the
intended trade-off, and it also makes the grant file a readable inventory of exactly what agents
in this repo are permitted to do.
