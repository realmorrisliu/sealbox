---
name: sealbox-skill
description: Operate, troubleshoot, and document the full Sealbox stack (server, CLI, tenant API, web UI, encryption model, migration, and operations). Use for key management, secret lifecycle, tenant administration, imports/exports, and deployment configuration.
---

# Sealbox

Use this skill for any question touching Sealbox project behavior across:

- Server architecture and HTTP/API behavior
- CLI command workflows for secrets, credentials, keys, tenants, and configuration
- Web UI capabilities and limitations
- Encryption workflow and security model
- Backup/migration/import-export workflows
- Operations and troubleshooting (health, startup behavior, auth issues, cleanup, migration reports)

Sealbox is intentionally split by security boundary:

- CLI (and future automation): owns key material, encryption/decryption, and full secret mutation workflows.
- Server: validates auth, persists encrypted bytes and metadata, and enforces tenant isolation.
- Web UI: supports operational visibility and safe secret metadata operations; decryption remains client-side in CLI unless browser encryption is explicitly added.

## Use this skill to

- Initialize and configure both CLI and server.
- Generate/register/rotate keys and validate key status.
- Store, list, read, delete, and version-manage secrets and credentials.
- Operate tenant token lifecycle (`/v2/admin/tenants` and `/v2/admin/tenants/:tenant_id/tokens`).
- Perform archive and bulk operations (`secret import`, `secret export`).
- Diagnose auth, token, TTL, and startup failures.
- Write migration and ops documentation for deployment runbooks.

## Source strategy

1. Verify current command/API shape via the in-repo docs when ambiguous.
2. Prefer CLI server-side behavior described in `/sealbox-skill/references/server-api.md` before giving API-level instructions.
3. Use `/sealbox-skill/references/cli-reference.md` for exact command workflows.
4. Use `/sealbox-skill/references/security-model.md` for any decision involving key handling or trust boundaries.

## Ground rules

- Treat private keys as secrets; they must never be committed, printed casually, or exposed to web/browser flows.
- `AUTH_TOKEN`/`SEALBOX_TOKEN` authenticate API calls only and are not encryption keys.
- Use `--output json` when giving machine-readable guidance for CI/CD or scripts.
- Assume `TTL` is seconds from creation time unless user states another unit.
- If a user asks for command examples that fail in their environment, confirm token scope (`v1` root token vs `v2` tenant token) before suggesting key/secret payload edits.

## Operating sequence (typical deployment)

1. Start server with SQLite path and auth token.
2. Verify readiness with `/healthz/ready`.
3. Configure CLI URL/token/key files.
4. Generate key pair, upload public key, and confirm key status.
5. Create secrets or credentials.
6. Retrieve/decrypt through CLI with private key in possession.
7. Rotate keys and validate active key state when required.

## Reference files

- `references/architecture-overview.md` - component responsibilities and data boundaries
- `references/server-api.md` - v1, v2, and admin endpoints
- `references/cli-reference.md` - command map and behavior
- `references/configuration.md` - env + config precedence and token file patterns
- `references/security-model.md` - threat model and encryption workflow
- `references/web-ui.md` - current UI capabilities and constraints
- `references/maintenance-and-troubleshooting.md` - operations and debug playbooks
- `references/archive-and-bulk-operations.md` - import/export formats and caveats
- `references/tenant-and-isolation.md` - tenant token behavior and legacy compatibility
