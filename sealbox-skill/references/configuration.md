# Configuration matrix

Use this as the first-pass mapping for all Sealbox configuration and precedence.

## Resolution order

For each setting:

1. Explicit CLI flags
2. Environment variables
3. Config file (`~/.config/sealbox/config.toml`)
4. Defaults

## Server config (`sealbox-server`)

Environment variables:

- `STORE_PATH`: SQLite DB path.
- `LISTEN_ADDR`: listen host and port (for example `127.0.0.1:8080`).
- `AUTH_TOKEN`: bearer token for protected routes.
- `AUTH_TOKEN_FILE`: path to file containing token.
- `LEGACY_V1_ENABLED`: compatibility mode flag for older v1 behavior.

Server startup requires valid DB path and token configuration depending on your deployment policy.

## CLI config (`sealbox-cli`)

Config file path:

- Defaults to `~/.config/sealbox/config.toml`
- Supports automatic `~` expansion for relative paths.

Key env vars and overrides:

- `SEALBOX_URL`: server URL.
- `SEALBOX_URL_FILE`: path containing server URL.
- `SEALBOX_TOKEN`: API token.
- `SEALBOX_TOKEN_FILE`: path containing token.
- `SEALBOX_PUBLIC_KEY_FILE`: local public key path.
- `SEALBOX_PRIVATE_KEY_FILE`: local private key path.
- `SEALBOX_API_VERSION`: `v1` or `v2`.
- `SEALBOX_OUTPUT_FORMAT`: `table`, `json`, or `yaml`.

File-based token and URL env values are read after loading plain vars and before fallback values.

## Runtime config files and tokens

- Both server and CLI support token-from-file styles for container/secret-manager patterns.
- Paths are resolved via filesystem reads at runtime; rotation in mounted files is supported by restarting/reloading clients.
- Key file paths support path expansion before use.

## API and behavior knobs

- `SEALBOX_API_VERSION=v2` switches CLI base endpoints to tenant mode.
- Tenant mode still uses Bearer token in `Authorization` header.
- v1/v2 separation is semantic and route-level, not only URL-level.

## Migration-related settings

- `LEGACY_V1_ENABLED` controls compatibility behavior in mixed environments.
- `sealbox-server migration-report --store-path <path>` reports legacy migration state for maintenance planning.

## Deployment tips

- In containerized environments prefer `_FILE` env vars and bind-mounted secrets.
- Keep token files with file permissions that prevent accidental read by other users.
- Keep config files out of version control; key files should never be committed.
