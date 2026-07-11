# Sealbox CLI reference

This covers behavior and intent for command usage. Prefer exact option names from `sealbox-cli --help` in the active version when ambiguity exists.

## Command tree

- `sealbox-cli config`
  - `show`
  - `set <key> <value>`
  - `init [--url ...] [--token ...] [--public-key ...] [--private-key ...] [--output ...]`
- `sealbox-cli key`
  - `generate`
  - `register`
  - `list`
  - `rotate`
  - `status`
- `sealbox-cli secret`
  - `set <key> [value] [--ttl <seconds>]`
  - `get <key> [--version <N>]`
  - `delete <key> [--version <N>]`
  - `list`
  - `history <key>`
  - `export <file>`
  - `import <file>`
- `sealbox-cli credential`
  - `set <key>`
  - `get <key>`
  - `list`
  - `history <key>`
  - `delete <key> [--version <N>]`
- `sealbox-cli password`
  - `generate`
- `sealbox-cli tenant`
  - `create <name>`
  - `list`
  - `get <tenant_id>`
  - `suspend <tenant_id>`
  - `resume <tenant_id>`
  - `token create <tenant_id>`
  - `token list <tenant_id>`
  - `token revoke <tenant_id> <token_id>`

## Common patterns

- Secret values and password values can be provided in three ways:
  - CLI prompt (hidden input) when value is omitted and stdin is a TTY.
  - Stdin pipe (`printf 'value' | sealbox-cli secret set key`) for scripts.
  - Direct positional argument in interactive terminal for convenience.
- Output format for tabular/automation use:
  - default is human table.
  - use `--output json` or `--output yaml` in supported commands.
- `credential` operations include username as plaintext metadata.
- `tenant` commands require root/admin token unless your deployment grants different policy.

## Configuration and environment integration

1. CLI config file (`~/.config/sealbox/config.toml`) stores URL, token, key paths, API version, output format.
2. Environment variables override config file values.
3. Explicit CLI flags override both env and config.

## Useful workflows

### Bring-up

1. `sealbox-cli config init --url <url> --token <token> --public-key ~/.config/sealbox/public.pem --private-key ~/.config/sealbox/private.pem`
2. `sealbox-cli key generate` (if keys are missing)
3. `sealbox-cli key register`
4. `sealbox-cli key status`

### Create and retrieve a secret

1. `sealbox-cli secret set api/client-id`
  - paste value when prompted.
2. `sealbox-cli secret get api/client-id`

### Expiring secret

1. `sealbox-cli secret set temp/token --ttl 3600`
2. After duration, secret metadata remains until cleanup/read; cleanup removes expired state.

### Bulk operations

- `sealbox-cli secret export backup.sealbox`
- `sealbox-cli secret import backup.sealbox`
- Import/export supports Sealbox archival formats used for migration and backup.

### Tenant onboarding

1. Admin flow: `tenant create`, `tenant token create`, copy token from response once.
2. Client flow: set API version to v2 and configure tenant token in CLI:
   - `SEALBOX_API_VERSION=v2`
   - `SEALBOX_TOKEN=<tenant_token>`
3. Operate secrets and credentials against tenant endpoints.

## Behavioral guardrails

- Never run export/import against incorrect store versions unless migration is intended.
- Prefer `key status` checks before large write batches to avoid dead-key writes.
- For scripted use, prefer `--output json` and explicit API version selection.
- `secret set` values are sensitive; avoid shell history leaks.
- `secret delete` without `--version` removes all versions for the key.
