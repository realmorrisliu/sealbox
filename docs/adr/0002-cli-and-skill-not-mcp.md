# Agent integration is a CLI plus a skill, not an MCP server

Agents interact with sealbox by running `sealbox` as a subprocess, guided by a skill document.
We are not shipping an MCP server.

An MCP server would be a long-running process that itself needs credentials configured — an odd
shape for the tool whose purpose is to stop credentials from being configured in files. A CLI
runs once and exits.

This covers how *agents* reach sealbox, and how humans do their routine work. Admin approvals are
the one exception: they go through a browser for the trusted display a terminal cannot give
(ADR 0009).

## Considered Options

MCP is the default assumption for agent integration in 2026, so the choice needs recording.
Rejected because:

- **Reach.** A CLI is usable from Claude Code, Cursor, Codex, shell scripts, and CI alike. MCP
  reaches only MCP clients — a strict subset, for a product whose premise is serving *any* agent.
- **Authorization granularity.** Host Bash allowlists can scope to `sealbox run *`. Tool-level
  MCP permissions are coarser.
- **A skill can state rules, a tool list cannot.** The skill tells the agent what *not* to do —
  "if no grant exists for the credential you need, ask a human to add one; do not go looking for
  the plaintext." That is a behavioural constraint, not an API description.

## Consequences

Discovery is worse: agents must be told sealbox exists via a skill, instead of enumerating tools
automatically. Accepted, because the skill has to exist anyway to carry the rules above.
