# Getting Started with Sealbox

This guide will walk you through setting up and using Sealbox for secure secret management.

## Prerequisites

- Rust 1.85+ (for building from source)
- A Unix-like system (Linux, macOS)

## Step 1: Build Sealbox

```bash
# Clone the repository
git clone https://github.com/realmorrisliu/sealbox.git
cd sealbox

# Build both server and CLI
cargo build --release
```

After building, you'll have:
- `./target/release/sealbox-server` - The server binary
- `./target/release/sealbox-cli` - The CLI tool

## Step 2: Start the Server

Create a directory for your Sealbox data and start the server:

```bash
# Create data directory
mkdir -p /var/lib/sealbox

# Set environment variables
export STORE_PATH=/var/lib/sealbox/sealbox.db
export AUTH_TOKEN=your-secure-token-here
export LISTEN_ADDR=127.0.0.1:8080

# Start the server
./target/release/sealbox-server
```

The server will output:
```
Sealbox server starting...
Listening on 127.0.0.1:8080
Database: /var/lib/sealbox/sealbox.db
```

## Step 3: Set Up the CLI

Initialize the CLI configuration with command-line parameters:

```bash
# Initialize with all parameters (recommended)
./target/release/sealbox-cli config init \
    --url http://localhost:8080 \
    --token your-secure-token-here \
    --public-key ~/.config/sealbox/public_key.pem \
    --private-key ~/.config/sealbox/private_key.pem \
    --output table

# Or initialize interactively (will prompt for missing values)
./target/release/sealbox-cli config init
```

This creates `~/.config/sealbox/config.toml` with your settings.

## Step 4: Generate Your Key Pair

Generate your RSA key pair for end-to-end encryption:

```bash
# Generate your private/public key pair
./target/release/sealbox-cli key generate
```

The keys will be automatically saved to:
- `~/.config/sealbox/private_key.pem` (permissions set to 600)
- `~/.config/sealbox/public_key.pem`

## Step 5: Register Your Public Key

Register your public key with the server:

```bash
# Register using your configuration
./target/release/sealbox-cli key register

# Verify the registration
./target/release/sealbox-cli key status
```

## Step 6: Store Your First Secret

Now you can securely store secrets:

```bash
# Store a database password
./target/release/sealbox-cli secret set db_password "my-super-secret-password"

# Store an API key with TTL (expires in 1 hour)
./target/release/sealbox-cli secret set api_key "sk-1234567890" --ttl 3600
```

## Step 7: Retrieve Secrets

```bash
# Get the database password
./target/release/sealbox-cli secret get db_password

# List all your secrets
./target/release/sealbox-cli secret list
```

## Understanding the Security Model

Sealbox uses **server-side envelope encryption** with **client-side decryption**:

1. **Your private key never leaves your machine** - Generated and stored locally
2. **CLI sends plaintext secrets** to the server over HTTPS
3. **Server encrypts using envelope encryption**:
   - Random AES-256-GCM key encrypts your secret
   - Your RSA public key encrypts the AES key
4. **Server stores encrypted data** - Cannot decrypt without your private key
5. **Only you can decrypt** secrets when retrieving them using your private key

**Key Point**: While secrets are sent as plaintext to the server, only you can decrypt the stored encrypted data.

## TTL (Time-To-Live) Features

Sealbox supports automatic expiration of secrets using TTL:

### How TTL Works
- **Set TTL**: Specify expiration time in seconds when storing secrets
- **Lazy Cleanup**: Expired secrets are deleted when you try to access them
- **Startup Cleanup**: Server removes expired secrets when it starts
- **Manual Cleanup**: Use admin API to batch-remove expired secrets

### TTL Examples
```bash
# Store a temporary API token (expires in 1 hour)
./target/release/sealbox-cli secret set api_token "temp-token-123" --ttl 3600

# Store a session key (expires in 30 minutes)
./target/release/sealbox-cli secret set session_key "session-abc" --ttl 1800

# Permanent secret (no TTL)
./target/release/sealbox-cli secret set permanent_key "never-expires"
```

### TTL Behavior Notes
- **Not Real-Time**: Expired secrets aren't deleted immediately when they expire
- **Access-Triggered**: Deletion happens when you try to retrieve the expired secret
- **Automatic**: No manual intervention needed for cleanup
- **Storage Efficient**: Expired data is eventually removed from disk

## Next Steps

- [CLI Reference](cli-reference.md) - Complete command documentation
- [Configuration](configuration.md) - Advanced configuration options
- [Security Guide](security.md) - Security best practices
- [API Reference](api-reference.md) - REST API documentation

## Troubleshooting

### Server won't start
- Check that the `STORE_PATH` directory exists and is writable
- Ensure the port in `LISTEN_ADDR` is not already in use
- Verify environment variables are set correctly

### CLI can't connect to server
- **Network Proxy Issues**: If you're using Surge, ClashX, or other proxy software, disable it for localhost connections or add localhost to bypass list
- Verify the server is running: `curl -H "Authorization: Bearer your-token" http://localhost:8080/v1/client-key`
- Check the URL and token in your configuration: `./target/release/sealbox-cli config show`

### Configuration Issues
- Use environment variables if needed: `SEALBOX_URL`, `SEALBOX_TOKEN`
- Config file location: `~/.config/sealbox/config.toml`
- Re-initialize if needed: `./target/release/sealbox-cli config init --force`

### TTL-Related Issues
- **Secret disappeared**: It may have expired, check if you set a TTL
- **Unexpected cleanup**: Server cleans expired secrets on startup
- **Storage not shrinking**: Use manual cleanup: `curl -X DELETE -H "Authorization: Bearer $TOKEN" $URL/v1/admin/cleanup-expired`
