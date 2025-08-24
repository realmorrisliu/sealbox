# Configuration Guide

This guide covers all configuration options for both Sealbox server and CLI.

## Server Configuration

The Sealbox server is configured entirely through environment variables.

### Required Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `STORE_PATH` | Path to SQLite database file | `/var/lib/sealbox/sealbox.db` |
| `AUTH_TOKEN` | Static bearer token for API authentication | `your-secure-token-123` |
| `LISTEN_ADDR` | Server listen address and port | `127.0.0.1:8080` |

### Optional Environment Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `RUST_LOG` | Logging level | `info` | `debug`, `warn`, `error` |

### Example Server Configuration

```bash
#!/bin/bash
# sealbox-server.sh

# Required settings
export STORE_PATH="/var/lib/sealbox/sealbox.db"
export AUTH_TOKEN="$(openssl rand -hex 32)"  # Generate random token
export LISTEN_ADDR="0.0.0.0:8080"

# Optional settings
export RUST_LOG="info"

# Create data directory
mkdir -p "$(dirname "$STORE_PATH")"

# Start server
exec ./target/release/sealbox-server
```

### Systemd Service Example

Create `/etc/systemd/system/sealbox.service`:

```ini
[Unit]
Description=Sealbox Secret Storage Service
After=network.target

[Service]
Type=simple
User=sealbox
Group=sealbox
WorkingDirectory=/opt/sealbox
ExecStart=/opt/sealbox/sealbox-server
Environment=STORE_PATH=/var/lib/sealbox/sealbox.db
Environment=AUTH_TOKEN=your-secure-token-here
Environment=LISTEN_ADDR=127.0.0.1:8080
Environment=RUST_LOG=info
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## CLI Configuration

The CLI uses a TOML configuration file with environment variable overrides and automatic path expansion.

### Configuration File Location

Default location: `~/.config/sealbox/config.toml`

Override with:
- `--config <path>` command line option
- `SEALBOX_CONFIG` environment variable

### Configuration File Format

```toml
# ~/.config/sealbox/config.toml

[server]
url = "http://localhost:8080"
token = "your-auth-token"

[keys]
private_key_path = "~/.config/sealbox/private_key.pem"
public_key_path = "~/.config/sealbox/public_key.pem"

[output]
format = "table"  # table, json, yaml

[cli]
timeout = 30  # seconds
```

### Configuration Initialization

Use the `config init` command to create your configuration file:

```bash
# Initialize with command-line parameters
sealbox-cli config init \
    --url http://localhost:8080 \
    --token your-secure-token \
    --public-key ~/.config/sealbox/public_key.pem \
    --private-key ~/.config/sealbox/private_key.pem \
    --output table

# Initialize interactively (prompts for values)
sealbox-cli config init

# Force overwrite existing configuration
sealbox-cli config init --force
```

### Configuration Options

#### `[server]` Section

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `url` | Sealbox server URL | `http://localhost:8080` | `https://sealbox.example.com` |
| `token` | Authentication token | (none) | `your-auth-token` |

#### `[keys]` Section

| Option | Description | Default | Notes |
|--------|-------------|---------|-------|
| `private_key_path` | Path to RSA private key | `~/.config/sealbox/private_key.pem` | Supports `~/` expansion |
| `public_key_path` | Path to RSA public key | `~/.config/sealbox/public_key.pem` | Supports `~/` expansion |

#### `[output]` Section

| Option | Description | Default | Values |
|--------|-------------|---------|--------|
| `format` | Default output format | `table` | `table`, `json`, `yaml` |

#### `[cli]` Section

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `timeout` | HTTP request timeout (seconds) | `30` | `60` |

### Environment Variable Overrides

CLI configuration can be overridden with environment variables:

| Environment Variable | Config Option | Example |
|---------------------|---------------|---------|
| `SEALBOX_URL` | `server.url` | `http://localhost:8080` |
| `SEALBOX_TOKEN` | `server.token` | `your-auth-token` |
| `SEALBOX_OUTPUT` | `output.format` | `json` |
| `SEALBOX_PRIVATE_KEY` | `keys.private_key_path` | `/path/to/private.pem` |
| `SEALBOX_PUBLIC_KEY` | `keys.public_key_path` | `/path/to/public.pem` |

### Configuration Priority

Configuration values are applied in this order (highest priority first):

1. Command-line arguments (`--url`, `--token`, etc.)
2. Environment variables (`SEALBOX_URL`, `SEALBOX_TOKEN`, etc.)
3. Configuration file (`~/.config/sealbox/config.toml`)
4. Built-in defaults

### Example Configurations

#### Development Configuration

```toml
[server]
url = "http://localhost:8080"
token = "dev-token-123"

[output]
format = "json"

[cli]
timeout = 10
```

#### Production Configuration

```toml
[server]
url = "https://sealbox.internal.company.com"
token = "prod-token-from-secret-manager"

[keys]
private_key_path = "/etc/sealbox/keys/private_key.pem"
public_key_path = "/etc/sealbox/keys/public_key.pem"

[output]
format = "table"

[cli]
timeout = 60
```

#### CI/CD Configuration

For automated environments, use environment variables:

```bash
# .env file or CI/CD variables
export SEALBOX_URL="https://sealbox.example.com"
export SEALBOX_TOKEN="${SEALBOX_SECRET_TOKEN}"
export SEALBOX_OUTPUT="json"
export SEALBOX_PRIVATE_KEY="/secrets/sealbox_private_key.pem"
```

## Security Considerations

### Server Security

1. **Token Security**: Use a cryptographically secure token:
   ```bash
   # Generate secure token
   openssl rand -hex 32
   ```

2. **File Permissions**: Secure the database file:
   ```bash
   chmod 600 /var/lib/sealbox/sealbox.db
   chown sealbox:sealbox /var/lib/sealbox/sealbox.db
   ```

3. **Network Security**: Bind to localhost or use TLS:
   ```bash
   # Localhost only
   export LISTEN_ADDR="127.0.0.1:8080"

   # Or use a reverse proxy with TLS
   export LISTEN_ADDR="127.0.0.1:8080"  # Behind nginx/apache
   ```

### CLI Security

1. **Key Protection**: Secure your private key:
   ```bash
   chmod 600 ~/.config/sealbox/private_key.pem
   ```

2. **Configuration Security**: Protect your config file:
   ```bash
   chmod 600 ~/.config/sealbox/config.toml
   ```

3. **Token Storage**: Avoid storing tokens in the config file in shared environments. Use environment variables instead.

## Troubleshooting Configuration

### Server Issues

```bash
# Check if server can read database path
ls -la $(dirname "$STORE_PATH")

# Verify server can bind to address
netstat -ln | grep 8080

# Check environment variables
env | grep -E "(STORE_PATH|AUTH_TOKEN|LISTEN_ADDR)"
```

### CLI Issues

```bash
# Check configuration file
sealbox-cli config show

# Test server connectivity
curl -H "Authorization: Bearer $SEALBOX_TOKEN" $SEALBOX_URL/v1/client-key

# Verify key files exist and are readable
ls -la ~/.config/sealbox/*.pem
```
