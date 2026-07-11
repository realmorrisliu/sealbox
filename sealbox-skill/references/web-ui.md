# Web UI reference (`sealbox-web`)

The web UI is a TanStack React app used for operational workflows and server visibility.

## Auth and session model

- Login flow captures server URL and token.
- Readiness probe (`/healthz/ready`) is used to validate connectivity during login.
- Session state is persisted via client storage and restored on app start.
- Auth guard blocks secret/key pages when not authenticated.
- Logout clears persisted auth state and routes back to login.

## Functional pages

- `/` - secret management page
  - list secrets
  - create secret entries
  - delete secrets (specific versions where supported)
  - optional TTL input during create
  - list filtering and status indicators
  - table/card view toggle
  - cleanup-expired action via API
- `/keys` - key status page
  - list registered keys
  - show active/current states
  - surface CLI-first guidance for key generation/rotation
- `/login` - authentication bootstrap route

## API integration surface used by UI

- v1 business routes for read/list/cleanup operations
- Health endpoints for connectivity feedback
- Readiness checks are used for status verification and UI rendering cues

## Data visibility constraints

- The UI is not a replacement for full secret decryption workflows.
- Secret values remain client-side decryption responsibilities in CLI; UI focuses on metadata operations and operational control.
- Credential metadata is shown with secure handling considerations.

## UX and language behavior

- Full i18n with English, Chinese, Japanese, German.
- Automatic language persistence and locale-aware date formatting.
- Server status is presented as compact icon state with response-time latency for operator feedback.

## Current limitations to avoid overpromising

- No browser private-key generation/encryption currently exposed in the UI layer.
- No API bypassing of CLI cryptographic responsibilities.
- Use CLI for workflows that must print plaintext secrets/campaign passwords.
