# CLI Reference

Complete reference for the Sealbox command-line interface.

## Global Options

All commands support these global options:

- `--config <path>` - Path to configuration file (default: `~/.config/sealbox/config.toml`)
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
- `--key-size <bits>` - RSA key size (default: 2048)
- `--force` - Overwrite existing keys

**Example:**
```bash
sealbox-cli key generate --key-size 4096
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

### `key list`

List all registered public keys on the server.

```bash
sealbox-cli key list [OPTIONS]
```

**Options:**
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

### `key status`

Show the status of your local keys and server registration.

```bash
sealbox-cli key status
```

### `key rotate`

Rotate to a new key pair (advanced operation).

```bash
sealbox-cli key rotate [OPTIONS]
```

**Options:**
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

## Secret Management Commands

### `secret set`

Store a secret with the given key.

```bash
sealbox-cli secret set <key> <value> [OPTIONS]
```

**Arguments:**
- `<key>` - Secret identifier
- `<value>` - Secret value (use `-` to read from stdin)

**Options:**
- `--ttl <seconds>` - Time-to-live in seconds
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

**Examples:**
```bash
# Store a simple secret
sealbox-cli secret set db_password "my-secret-password"

# Store with TTL (expires in 1 hour)
sealbox-cli secret set temp_token "abc123" --ttl 3600

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

**Examples:**
```bash
# Get latest version
sealbox-cli secret get db_password

# Get specific version
sealbox-cli secret get db_password --version 2
```

### `secret list`

List all your secrets (metadata only, no values).

```bash
sealbox-cli secret list [OPTIONS]
```

**Options:**
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

### `secret delete`

Delete a secret or specific version.

```bash
sealbox-cli secret delete <key> [OPTIONS]
```

**Arguments:**
- `<key>` - Secret identifier

**Options:**
- `--version <version>` - Specific version to delete (default: all versions)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

**Examples:**
```bash
# Delete all versions
sealbox-cli secret delete old_password

# Delete specific version
sealbox-cli secret delete old_password --version 1
```

### `secret import`

Import secrets from a file.

```bash
sealbox-cli secret import [OPTIONS]
```

**Options:**
- `--file <path>` - JSON file containing secrets
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
sealbox-cli secret export [OPTIONS]
```

**Options:**
- `--file <path>` - Output file path
- `--format <format>` - Output format: `json` (default)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

## Legacy Commands

### `master-key create`

Legacy command for key generation and registration in one step.

```bash
sealbox-cli master-key create [OPTIONS]
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
- `SEALBOX_CONFIG` - Configuration file path
- `SEALBOX_OUTPUT` - Default output format

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Authentication error
- `3` - Network/connection error
- `4` - File/configuration error