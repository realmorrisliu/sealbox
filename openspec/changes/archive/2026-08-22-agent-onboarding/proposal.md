# Agent onboarding

## Why

Everything sealbox needs is now built, and nothing tells an agent how to use it. The gap is not
documentation — `docs/cli-reference.md` is thorough — it is that an agent reaching for a secret
does not know sealbox exists, does not know it must never ask for a value, and will reach for
`kubectl create secret` because that is what its training says.

The design has said from the start that integration is a **skill plus a CLI tool, not an MCP
server** (ADR 0002). This is the skill.

The thing worth getting right is not the command list. It is the two habits that make sealbox
work at all:

1. **Never ask for a value.** There is no command that prints one, and an agent that keeps trying
   will conclude the tool is broken rather than that it is holding it wrong.
2. **Draft, then hand over.** An agent writes the grant and submits it; a human signs for it. An
   agent that waits for approval quietly is useful; one that retries or works around the refusal
   is worse than no integration.

## What changes

- A skill file describing what an agent may do, what it must hand to a human, and why.
- The examples become the skill's template library rather than a separate thing — they already
  exist and are already real.
- A `README.md` section pointing an agent at the skill, since that is where one looks first.

## Non-goals

- **No MCP server** (ADR 0002).
- **No new commands.** If the skill needs a command that does not exist, that is a finding about
  the CLI, not something to paper over in prose.
