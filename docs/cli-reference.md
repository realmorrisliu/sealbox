# CLI Reference

Complete reference for the Sealbox command-line interface.

## Global Options

All commands support these global options:

- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)
- `--public-key <path>` - Public key file path (overrides config)
- `--private-key <path>` - Private key file path (overrides config)
- `--output <format>` - Output format: `table`, `json`, `yaml` (default: `table`)
- `--help` - Show help information
- `--version` - Show version information

## Configuration Commands

### `config init`

Initialize CLI configuration interactively or with command-line parameters.

```bash
sealbox-cli config init [OPTIONS]
```

**Options:**
- `--url <url>` - Server URL
- `--token <token>` - Authentication token
- `--public-key <path>` - Public key file path
- `--private-key <path>` - Private key file path
- `--output <format>` - Output format: `table`, `json`, `yaml`
- `--force` - Overwrite existing configuration file

**Examples:**
```bash
# Initialize with all parameters
sealbox-cli config init \
    --url http://localhost:8080 \
    --token your-token \
    --public-key ~/.config/sealbox/public_key.pem \
    --private-key ~/.config/sealbox/private_key.pem \
    --output table

# Initialize interactively (prompts for missing values)
sealbox-cli config init

# Force overwrite existing configuration
sealbox-cli config init --force
```

Creates `~/.config/sealbox/config.toml` with your settings.

### `config show`

Display current configuration.

```bash
sealbox-cli config show
```

## Key Management Commands

### `key generate`

Generate a new RSA key pair for encryption.

```bash
sealbox-cli key generate [OPTIONS]
```

**Options:**
- `--public-key-path <path>` - Public key file output path (overrides config)
- `--private-key-path <path>` - Private key file output path (overrides config)
- `--force` - Overwrite existing keys

**Example:**
```bash
sealbox-cli key generate --public-key-path ~/.config/sealbox/public.pem --private-key-path ~/.config/sealbox/private.pem
```

### `key register`

Register your public key with the Sealbox server.

```bash
sealbox-cli key register [OPTIONS]
```

**Options:**
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

**Example:**
```bash
sealbox-cli key register --url http://localhost:8080 --token my-token
```

### `key status`

Show the status of your local keys and server registration.

```bash
sealbox-cli key status
```

### `key rotate`

Rotate to a new key pair using client-side rotation (private key stays local).

```bash
sealbox-cli key rotate [OPTIONS]
```

Behavior:
- Generates a new key pair locally
- Lists all secret associations for the current client
- For each association, decrypts the DataKey locally and re-encrypts it with the new public key
- Uploads the new encrypted DataKey per association
- Updates the client’s public key on the server

Server APIs used by this command:
- `GET /v1/clients/{client_id}/secrets`
- `PUT /v1/secrets/{key}/permissions/{client_id}/data-key`
- `PUT /v1/clients/{client_id}/public-key`

**Options:**
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

### `up`

One-step setup: generate keys (if missing) and register this device as a client.

```bash
sealbox-cli up [--name <name>] [--description <desc>] [--enroll]
```

Behavior:
- If keys don’t exist: generate RSA key pair locally
- If not registered: register client and store `client_id` in config
- `--enroll`: start enrollment code flow (prints code + verify URL; waits for approval)

### `client` commands

```bash
sealbox-cli client register [--name <name>] [--description <desc>]
sealbox-cli client list
sealbox-cli client disable <client-id>
sealbox-cli client rename <client-id> <name> [--description <desc>]
sealbox-cli client status
```

Behavior:
- `register`: call `/v1/clients` and persist `keys.client_id` on success
- `list`: show id/name/status/created_at/last_used_at
- `disable`: set status=Disabled
- `rename`: update name/description
- `status`: show current client_id, key paths, associations count

## Secret Management Commands

### `secret set`

Store a secret with the given key.

```bash
sealbox-cli secret set <key> <value> [--ttl <seconds>] [--clients <id-or-name>[,<id-or-name>...]]
```

**Arguments:**
- `<key>` - Secret identifier
- `<value>` - Secret value (use `-` to read from stdin)

**Options:**
- `--ttl <seconds>` - Time-to-live in seconds (expires after creation time)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)
- `--clients <...>` - Multi-client authorization; accepts UUIDs or client names

**TTL Behavior:**
- Expired secrets are automatically deleted when accessed (lazy cleanup)
- Server also cleans expired secrets on startup
- Use admin cleanup endpoint for immediate batch removal

**Examples:**
```bash
# Store a simple secret (permanent)
sealbox-cli secret set db_password "my-secret-password"

# Store with TTL (expires in 1 hour = 3600 seconds)
sealbox-cli secret set temp_token "abc123" --ttl 3600

# Store session data (expires in 30 minutes)
sealbox-cli secret set session_data "user-session-123" --ttl 1800

# Store short-lived API key (expires in 5 minutes)
sealbox-cli secret set quick_key "temp-key-456" --ttl 300

# Read secret from stdin
echo "my-secret" | sealbox-cli secret set api_key -
```

### `secret get`

Retrieve and decrypt a secret.

```bash
sealbox-cli secret get <key> [OPTIONS]
```

**Arguments:**
- `<key>` - Secret identifier

**Options:**
- `--version <version>` - Specific version to retrieve (default: latest)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

**TTL Behavior:**
- If the secret has expired, it will be automatically deleted and you'll get a "Secret not found" error
- This is the lazy cleanup mechanism in action

Notes:
- CLI automatically sends `X-Client-ID` (from config) so multi-client secrets return the correct DataKey for this device

**Examples:**
```bash
# Get latest version
sealbox-cli secret get db_password

# Get specific version
sealbox-cli secret get db_password --version 2

# Expired secret will return "Secret not found"
sealbox-cli secret get expired_token
```

### `secret permissions`

View secret access permissions.

```bash
sealbox-cli secret permissions <key>
```

### `secret revoke`

Revoke a client’s permission on a secret.

```bash
sealbox-cli secret revoke <key> --client <id-or-name>
```

The `<id-or-name>` can be a client UUID or a client name; names are resolved via `client list`.

### `secret import`

Import secrets from a file.

```bash
sealbox-cli secret import <file> [--format json]
```

**Options:**
- `<file>` - JSON file containing secrets (positional)
- `--format <format>` - Input format: `json` (default)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

**Input File Format (JSON):**
```json
{
  "secrets": {
    "db_password": "secret-value-1",
    "api_key": "secret-value-2"
  }
}
```

**Example:**
```bash
sealbox-cli secret import --file secrets.json
```

### `secret export`

Export secrets to a file (requires local decryption).

```bash
sealbox-cli secret export [<file|->] [--keys <glob>] [--format env|json|yaml|shell] [--prefix <NAME>]
```

**Options:**
- `<file>` - Output file path or `-` for stdout (default: `-`)
- `--keys <glob>` - Filter keys by glob pattern (e.g., `db_*`)
- `--format <format>` - Output: `env` (default), `shell`, `json`, `yaml`
- `--prefix <NAME>` - Prefix for env var names (env/shell formats)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

## TTL and Administration

### TTL (Time-To-Live) Overview

TTL allows secrets to automatically expire and be deleted:

**How it works:**
- Set TTL in seconds when storing secrets with `--ttl <seconds>`
- Expired secrets are deleted when accessed (lazy cleanup)
- Server cleans expired secrets on startup
- Manual cleanup available via admin API

**Use cases:**
- **Temporary tokens**: API keys that should expire quickly
- **Session data**: User sessions with automatic timeout
- **One-time secrets**: Passwords that should be short-lived
- **Development**: Temporary configurations for testing

### Manual Cleanup (Admin)

While the CLI doesn't have a direct admin command, you can manually trigger cleanup:

```bash
# Using curl to trigger manual cleanup
curl -X DELETE \
  -H "Authorization: Bearer your-token" \
  http://localhost:8080/v1/admin/cleanup-expired

# Response shows cleanup statistics
{
  "deleted_count": 15,
  "cleaned_at": 1640995200
}
```

## Legacy Commands

### `client-key create`

Legacy command for key generation and registration in one step.

```bash
sealbox-cli client-key create [OPTIONS]
```

**Options:**
- `--url <url>` - Server URL
- `--token <token>` - Authentication token
- `--public-key-path <path>` - Public key file path
- `--private-key-path <path>` - Private key file path

## Output Formats

### Table Format (Default)

Human-readable table output:
```
┌─────────────┬─────────┬─────────────────────┐
│ Key         │ Version │ Created             │
├─────────────┼─────────┼─────────────────────┤
│ db_password │ 1       │ 2024-01-15 10:30:45 │
│ api_key     │ 2       │ 2024-01-15 11:15:20 │
└─────────────┴─────────┴─────────────────────┘
```

### JSON Format

Machine-readable JSON output:
```json
{
  "secrets": [
    {
      "key": "db_password",
      "version": 1,
      "created": "2024-01-15T10:30:45Z"
    }
  ]
}
```

### YAML Format

YAML output:
```yaml
secrets:
  - key: db_password
    version: 1
    created: 2024-01-15T10:30:45Z
```

## Environment Variables

CLI commands can be configured using environment variables:

- `SEALBOX_URL` - Server URL
- `SEALBOX_TOKEN` - Authentication token
- `SEALBOX_OUTPUT_FORMAT` - Default output format (`table`, `json`, `yaml`)
- `SEALBOX_PRIVATE_KEY` - Private key file path
- `SEALBOX_PUBLIC_KEY` - Public key file path

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Authentication error
- `3` - Network/connection error
- `4` - File/configuration error
### `secret permissions`

View secret access permissions.

```bash
sealbox-cli secret permissions <key>
```

### `secret revoke`

Revoke a client’s permission on a secret.

```bash
sealbox-cli secret revoke <key> --client <id-or-name>
```

The `<id-or-name>` can be a client UUID or a client name; names are resolved via `client list`.
