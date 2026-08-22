# Decisions

Each records what was decided, what was rejected, and why. Several supersede or amend earlier
ones — the reversals are kept rather than rewritten, because knowing a decision was reconsidered
is part of its weight.

| # | Decision | Turns on |
|---|---|---|
| [0001](0001-broker-over-e2ee.md) | Broker model over end-to-end encryption | An agent holding a private key is worse than one holding a secret |
| [0002](0002-cli-and-skill-not-mcp.md) | A CLI plus a skill, not an MCP server | Reach, authorisation granularity, and that a skill can state rules |
| [0003](0003-named-grants-not-free-form-commands.md) | Agents invoke named grants, never compose commands | Withholding plaintext alone does not survive prompt injection |
| [0004](0004-no-web-ui.md) | No web UI — *amended by 0009* | The interface is the CLI; the approval page is a ceremony, not an interface |
| [0005](0005-k8s-is-not-the-primary-scenario.md) | Operating Kubernetes is not the scenario; supplying its Secrets is | RBAC already solves capability, server-side and unavoidably |
| [0006](0006-out-of-band-secret-supply.md) | Secrets pushed out of band; no cluster controller | Declarative secrets require an operator; that cost was measured and reversed |
| [0007](0007-adapters-first-scripts-as-escape-hatch.md) | Adapters first, scripts as escape hatch; sealbox generates values | Adapters narrow capability; scripts do not |
| [0008](0008-runner-is-the-only-executor.md) | The runner is the only executor; the CLI is a remote control | A hosted server cannot reach a VPC, and an agent's host must not hold plaintext |
| [0009](0009-admin-authenticates-with-passkeys.md) | Admin authenticates with passkeys, not a stored token | Nothing on disk to steal, and a browser is the only trusted display |
| [0010](0010-recovery-via-keypair-not-a-copied-key.md) | Recovery via a recovery keypair, not a copied key | A master key that appears in logs has leaked |
| [0011](0011-rotation-uses-dual-credentials-and-a-linear-chain.md) | Rotation creates a second credential; a linear chain | Mutating in place guarantees a window where production is down |
| [0012](0012-no-backward-compatibility-before-first-release.md) | No backward compatibility before the first release | Migration code for a database nobody has is pure carrying cost |
| [0013](0013-automation-first.md) | Automate first; a human only widens authority | Reducing human intervention without a boundary would optimise away the one act the product rests on |
