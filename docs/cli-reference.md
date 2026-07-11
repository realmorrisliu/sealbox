# CLI Reference

Complete reference for the Sealbox command-line interface.

## Server Maintenance Command

Inspect a pre-v2 database without modifying it:

```bash
sealbox-server migration-report --store-path /var/lib/sealbox/sealbox.db
```

The JSON report includes schema version, migration requirement, key and secret
counts, empty legacy namespace rows, and orphaned secret references. Normal
startup creates `<STORE_PATH>.pre-tenant-v2.bak` before migrating populated
legacy data.

## Global Options

All commands support these global options:

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

- `--public-key-path <path>` - Public key output path
- `--private-key-path <path>` - Private key output path
- `--force` - Overwrite existing keys

**Example:**
```bash
sealbox-cli key generate --force
```

### `key register`

Register your public key with the Sealbox server.

```bash
sealbox-cli key register [OPTIONS]
```

**Options:**
- `--new-key-id <uuid>` - New registered master key id
- `--old-key-id <uuid>` - Old active master key id

The CLI decrypts existing encrypted data keys locally with the old private key and sends only rewrapped encrypted data keys to the server.

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
sealbox-cli secret set <key> [value] [OPTIONS]
```

**Arguments:**
- `<key>` - Secret identifier
- `[value]` - Optional secret value. If omitted, the CLI reads from a hidden prompt when attached to a TTY, or from stdin when piped.

**Options:**
- `--ttl <seconds>` - Time-to-live in seconds (expires after creation time)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

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

# Read secret from piped stdin
printf '%s\n' "my-secret" | sealbox-cli secret set api_key
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

**Examples:**
```bash
# Get latest version
sealbox-cli secret get db_password

# Get specific version
sealbox-cli secret get db_password --version 2

# Expired secret will return "Secret not found"
sealbox-cli secret get expired_token
```

### `secret list`

List all your secrets (metadata only, no values).

```bash
sealbox-cli secret list [OPTIONS]
```

**Options:**
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

### `secret history`

List retained version metadata for one secret. Secret values and encrypted payload bytes are not returned.

```bash
sealbox-cli secret history <key> [OPTIONS]
```

**Arguments:**
- `<key>` - Secret identifier

**Options:**
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

### `secret delete`

Delete a secret and all stored versions by default, or delete one specific version.

```bash
sealbox-cli secret delete <key> [OPTIONS]
```

**Arguments:**
- `<key>` - Secret identifier

**Options:**
- `--version <version>` - Delete only this specific version
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)

**Examples:**
```bash
# Delete all versions
sealbox-cli secret delete old_password

# Delete specific version
sealbox-cli secret delete old_password --version 1
```

### `secret export`

Export latest secret versions to a Sealbox encrypted archive. The CLI decrypts each selected secret locally, writes a tar payload in memory, encrypts that tar payload with AES-256-GCM, and encrypts the archive data key with the configured public key.

```bash
sealbox-cli secret export <file> [OPTIONS]
```

**Options:**
- `--keys <text>` - Export only keys containing this substring
- `--format <format>` - Archive format: `encrypted-tar` or `sealbox-v1` (default: `encrypted-tar`)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)
- `--public-key <path>` - Public key used to encrypt the archive data key
- `--private-key <path>` - Private key used to decrypt exported secrets

**Archive format:**
- Outer file: JSON envelope with `envelope_version`, cipher names, and base64 ciphertext fields
- Encrypted payload: tar archive containing `manifest.json` and `secrets.json`
- `manifest.json`: includes `format_version` so newer importers can migrate older payload structures
- Output file permissions: `0600` on Unix when the CLI creates the file

**Examples:**
```bash
# Export all latest secret versions
sealbox-cli secret export backups/sealbox-export.tar.enc

# Export only matching keys
sealbox-cli secret export backups/db-secrets.tar.enc --keys db/
```

### `secret import`

Import secrets from a Sealbox encrypted archive. The CLI decrypts the archive locally with the configured private key, reads the tar manifest version, migrates supported old structures, and stores each secret through the normal client-side encrypted write path.

```bash
sealbox-cli secret import <file> [OPTIONS]
```

**Options:**
- `--format <format>` - Archive format: `encrypted-tar` or `sealbox-v1` (default: `encrypted-tar`)
- `--url <url>` - Server URL (overrides config)
- `--token <token>` - Authentication token (overrides config)
- `--private-key <path>` - Private key used to decrypt the archive data key

**Import behavior:**
- Imported records become new versions in the destination database
- Plaintext metadata is preserved
- Future `expires_at` values are converted back into TTL seconds at import time
- Already expired records are skipped
- Unsupported envelope or archive format versions are rejected

**Example:**
```bash
sealbox-cli secret import backups/sealbox-export.tar.enc
```

## Password Generation Commands

Password generation happens locally in the CLI. The standalone command prints generated passwords to stdout. Credential generation stores the password directly through the encrypted credential flow.

### `password generate`

Generate strong passwords or alphanumeric random-ID-style values.

```bash
sealbox-cli password generate [OPTIONS]
```

**Options:**
- `--length <number>` - Password length (default: 24)
- `--count <number>` - Number of passwords to generate (default: 1)
- `--alphanumeric` - Use ASCII letters and digits only
- `--no-symbols` - Exclude symbols
- `--no-numbers` - Exclude numbers
- `--no-uppercase` - Exclude uppercase letters
- `--no-lowercase` - Exclude lowercase letters
- `--exclude-ambiguous` - Exclude ambiguous characters such as `O`, `0`, `I`, `l`, and `1`

**Examples:**
```bash
# Generate one broad-character password
sealbox-cli password generate --length 32

# Generate an alphanumeric random-id-style value
sealbox-cli password generate --alphanumeric --length 40

# Generate several values
sealbox-cli password generate --count 5 --exclude-ambiguous
```

## Credential Commands

Credential commands store username/password pairs as encrypted JSON secret values. The username is also stored as plaintext metadata so credentials can be listed and filtered without decrypting every record.

Sealbox retains only the latest 10 versions for each credential key. Older credential versions are pruned automatically when a new credential version is saved.

### `credential set`

Store a username/password credential. The password is read from a hidden prompt when attached to a TTY, from stdin when piped, or generated locally with `--generate-password`.

```bash
sealbox-cli credential set <key> --username <username> [OPTIONS]
```

**Options:**
- `--username <username>` - Username to store in encrypted data and plaintext metadata
- `--ttl <seconds>` - Time-to-live in seconds
- `--generate-password` - Generate a strong password locally instead of prompting or reading stdin
- `--show-password` - Print the generated password after it is saved
- `--length <number>` - Generated password length (default: 24)
- `--alphanumeric` - Generate only ASCII letters and digits
- `--no-symbols` - Exclude symbols from generated passwords
- `--no-numbers` - Exclude numbers from generated passwords
- `--no-uppercase` - Exclude uppercase letters from generated passwords
- `--no-lowercase` - Exclude lowercase letters from generated passwords
- `--exclude-ambiguous` - Exclude ambiguous generated characters such as `O`, `0`, `I`, `l`, and `1`

**Example:**
```bash
sealbox-cli credential set db/postgres --username app_user

# Generate and store a password without printing it
sealbox-cli credential set db/postgres --username app_user --generate-password

# Generate an alphanumeric password for systems that reject symbols
sealbox-cli credential set api/service \
  --username service_user \
  --generate-password \
  --alphanumeric \
  --length 40

# Non-interactive stdin
printf '%s\n' "db-password" | sealbox-cli credential set db/postgres --username app_user
```

### `credential get`

Retrieve and decrypt a credential.

```bash
sealbox-cli credential get <key> [OPTIONS]
```

**Options:**
- `--version <version>` - Specific version to retrieve

### `credential list`

List credentials using plaintext metadata. Passwords are not included.

```bash
sealbox-cli credential list [OPTIONS]
```

**Options:**
- `--name <text>` - Filter by credential name/key substring
- `--key <text>` - Alias for `--name`
- `--username <text>` - Filter by username substring
- `--query <text>` - Filter by credential name/key or username substring

Search filters are case-insensitive substring matches. If multiple specific filters are provided, all of them must match. `--query` matches either credential name/key or username.

### `credential history`

List retained version metadata for one credential, including version numbers and plaintext username metadata. Passwords are not included.

```bash
sealbox-cli credential history <key>
```

**Example:**
```bash
sealbox-cli credential history db/postgres --output json
```

### `credential delete`

Delete a credential and all stored versions.

```bash
sealbox-cli credential delete <key>
```

**Example:**
```bash
sealbox-cli credential delete db/postgres
```

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
- `SEALBOX_TOKEN_FILE` - File containing authentication token
- `SEALBOX_PUBLIC_KEY` - Public key path
- `SEALBOX_PRIVATE_KEY` - Private key path
- `SEALBOX_PUBLIC_KEY_FILE` - Mounted public key file path
- `SEALBOX_PRIVATE_KEY_FILE` - Mounted private key file path
- `SEALBOX_OUTPUT_FORMAT` - Default output format

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Authentication error
- `3` - Network/connection error
- `4` - File/configuration error
## Tenant Administration Commands

Tenant commands require the server root token. New token values are never
printed; `--token-file` must name a file that does not already exist.

```bash
sealbox-cli tenant create --display-name app-a --token-file /secure/app-a/token
sealbox-cli tenant list
sealbox-cli tenant get <tenant-id>
sealbox-cli tenant suspend <tenant-id>
sealbox-cli tenant resume <tenant-id>
sealbox-cli tenant token create <tenant-id> --label rotated --token-file /secure/app-a/token-2
sealbox-cli tenant token list <tenant-id>
sealbox-cli tenant token revoke <tenant-id> <token-id>
```

For tenant key and secret commands, set `SEALBOX_API_VERSION=v2`, select the
tenant token file, and select that tenant's public/private key files.
