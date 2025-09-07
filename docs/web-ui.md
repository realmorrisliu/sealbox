# Sealbox Web UI — Admin Console

The Sealbox Web UI is the management console for administrators and operators. It provides visual oversight, approvals, and permission controls for multi‑client secrets. It does not decrypt or consume secrets; that is the responsibility of sealbox‑cli running on client devices. Think of it like Tailscale’s admin console: approve devices, manage access, audit activity — while clients perform the actual secure operations.

## Role Separation (Server · CLI · Web)

- Server (`sealbox-server`): API + storage control plane. Stores encrypted data and authorization relationships. Never sees plaintext or DataKeys.
- CLI (`sealbox-cli`): Runs on devices. Generates/rotates keys, enrolls/registers as clients, encrypts/decrypts locally, and consumes secrets.
- Web (`sealbox-web`): Admin UI. Approves enrollments, manages clients, defines permissions at secret creation time, monitors status, and audits activity. Never handles plaintext.

Key properties aligned with the multi-client architecture:
- One secret = one DataKey; the DataKey is encrypted per authorized client.
- Permissions are set at creation time; you can revoke later, but cannot add new clients to past versions’ DataKeys on the server side.
- Client isolation: each client has an independent key pair and identity.

## Primary Capabilities (Management Focus)

- Secrets overview: list, filter, and monitor TTL/status across all secrets.
- Multi‑client secret creation: select authorized clients when creating a secret; Web never handles plaintext beyond metadata entry from the browser input field.
- Permission visibility and revocation: view which clients are authorized for a secret; revoke client access per secret and visualize impact.
- Client/device management: list clients (devices), rename/update description, toggle status (Active/Disabled/Retired), and see last used time.
- Enrollment approvals: approve pending `sealbox-cli up --enroll` requests with a verification code; optionally set client name/description.
- Auditing and activity: view operational history (create, revoke, cleanup), high‑level usage indicators, and health checks.
- Admin maintenance: trigger expired‑secret cleanup; manage defaults (e.g., TTL hints), and show CLI guidance for key tasks.

Non‑goals:
- No plaintext viewing/decryption in the browser.
- No client‑side crypto operations in the Web UI.
- No secret consumption/output; that lives in CLI.

## Navigation & Pages

- Dashboard: high‑level stats, expiring soon, recent activity, server health.
- Secrets: searchable table/card views; create secret (with multi‑client authorization), delete version; decrypt hints that redirect users to CLI usage.
- Permissions: per‑secret permissions drawer and a matrix view (Secrets × Clients) for audits and bulk revokes.
- Clients: list/rename/describe/toggle status; view each client’s associations (read‑only) and last used time.
- Enrollments: see pending codes, approve with optional name/description, verify expiry timers.
- Audit: operation log (create/revoke/cleanup, status changes) with filters and time range.
- Settings: server URL/token management (local to browser), defaults and UI preferences, CLI integration instructions.

## Core Workflows

- Onboard a new device (Tailscale‑style):
  - Device runs `sealbox-cli up --enroll` to get a short code + verify URL.
  - Admin opens Web → Enrollments, locates the code, and approves with a name/description.
  - CLI finishes enrollment and registers the client key.

- Create a multi‑client secret:
  - Web → Secrets → Create → enter key/value and TTL; select authorized clients.
  - Web submits once; server stores a single `encrypted_data` and multiple `encrypted_data_key` records.
  - Later, revoke client(s) on the permissions panel if needed.

- Revoke a client’s access:
  - From a secret’s permissions panel or the matrix view, remove a client.
  - Server deletes the corresponding `encrypted_data_key` association; others remain valid.

- Rotate a client key (client‑side):
  - From Clients page, open CLI instructions to run `sealbox-cli key rotate`.
  - CLI re‑encrypts each DataKey and updates the client public key; Web reflects the change in associations.

## Current Implementation Snapshot (sealbox-web)

- Implemented:
  - Secrets list with TTL/status indicators and delete actions.
  - Create Secret dialog with multi‑client selection for authorized clients.
  - Clients list (Keys page) with status badges and CLI guidance for register/rotate.
  - Auth with token + server URL; internationalization in multiple languages.

- Next up to match the architecture doc:
  - Enrollments page for approval flow (`POST/GET/PUT /v1/enroll`).
  - Migrate client management API usage from legacy `/v1/client-key` to `/v1/clients` family.
  - Permission panels and the matrix view, plus revoke actions.
  - Audit log surface and simple health/status panel on Dashboard.

## Quick Start

### Prerequisites

- Node.js 18+
- pnpm (recommended) or npm
- Running sealbox-server instance

### Installation

```bash
# Navigate to web UI directory
cd sealbox-web

# Install dependencies
pnpm install

# Start development server
pnpm run dev

# Open browser
open http://localhost:3000
```

### First Time Setup

1. **Start sealbox-server** with CORS enabled (automatic in debug mode)
2. **Access Web UI** at http://localhost:3000
3. **Login** with your server URL and AUTH_TOKEN:
   - Server URL: `http://localhost:8080`
   - Token: Your `AUTH_TOKEN` environment variable value
4. **Choose your language** - Interface available in English, Chinese, Japanese, and German

## Project Structure

```
sealbox-web/
├── src/
│   ├── components/
│   │   ├── auth/
│   │   │   └── auth-guard.tsx      # Route protection
│   │   ├── layout/
│   │   │   └── main-layout.tsx     # Main app layout
│   │   └── ui/                     # shadcn/ui components
│   │       └── language-selector.tsx # Language switching
│   ├── hooks/
│   │   └── use-api.ts              # API integration hooks
│   ├── lib/
│   │   ├── api.ts                  # API client
│   │   ├── i18n.ts                 # Internationalization config
│   │   ├── types.ts                # TypeScript definitions
│   │   ├── utils.ts                # Utilities
│   │   └── query-client.ts         # React Query config
│   ├── locales/
│   │   ├── en.json                 # English translations
│   │   ├── zh.json                 # Chinese translations
│   │   ├── ja.json                 # Japanese translations
│   │   └── de.json                 # German translations
│   ├── routes/
│   │   ├── __root.tsx              # Root route + providers
│   │   ├── index.tsx               # Secret list page
│   │   └── login.tsx               # Login page
│   └── stores/
│       ├── auth.ts                 # Authentication state
│       └── config.ts               # App configuration
├── components.json                 # shadcn/ui config
├── package.json
├── tailwind.config.ts
├── tsconfig.json
└── vite.config.ts
```

## Features in Detail

### Authentication & Security

- Token‑based Bearer auth; server URL + token stored locally in the browser.
- Connection test on login; automatic logout on auth failures.
- Web never decrypts secrets; plaintext never leaves clients.

### Secrets Management (Metadata)

- List secrets with version, timestamps, and TTL indicators:
  - 🟢 >24h remaining, 🟡 <24h, 🔴 <1h.
- Create secret with optional TTL and multi‑client authorization.
- Delete a specific version with confirmation; expired items show countdown.
- Decrypt hints link users to CLI commands for retrieval/consumption.

### Clients (Devices)

- List all clients with id/name/status/created/last used.
- Rename/update description; toggle status (Active/Disabled/Retired).
- Show associations count for rotation planning (via CLI flow).

### Enrollments

- View pending enrollment codes and expiry timers.
- Approve with optional name/description; failed/expired codes clearly indicated.

### Permissions

- Per‑secret pane: list authorized clients and revoke individually.
- Matrix view (Secrets × Clients) for audits and multi‑select revoke.

### Admin & Audit

- Trigger expired‑secret cleanup and view results.
- Simple operational activity feed (create/revoke/cleanup, status changes).
- Server health/readiness checks with response time.

## Configuration

### Environment Variables

```bash
# Vite environment variables
VITE_DEFAULT_SERVER_URL=http://localhost:8080
VITE_APP_NAME=Sealbox
```

### Development

```bash
# Development server
pnpm run dev

# Build for production
pnpm run build

# Preview production build
pnpm run preview
```

## API Integration

Primary endpoints used by the Web UI:

- Secrets
  - `GET /v1/secrets` — List secrets (metadata).
  - `GET /v1/secrets/{key}?version=N` — Secret details (metadata, no decryption in Web).
  - `PUT /v1/secrets/{key}` — Create secret with payload `{ secret, ttl?, authorized_clients?[] }`.
  - `DELETE /v1/secrets/{key}?version=N` — Delete a specific version.
  - `GET /v1/secrets/{key}/permissions` — View secret permissions.
  - `DELETE /v1/secrets/{key}/permissions/{client_id}` — Revoke permission.

- Clients
  - `GET /v1/clients` — List registered clients (devices).
  - `PUT /v1/clients/{client_id}/status` — Enable/disable/retire client.
  - `PUT /v1/clients/{client_id}/name` — Rename/update description.
  - `GET /v1/clients/{client_id}/secrets` — List associations (for rotation planning; surfaced read‑only in Web).

- Enrollment
  - `POST /v1/enroll` — Begin enrollment (CLI‑initiated; code + verify URL).
  - `GET /v1/enroll/{code}` — Check status (CLI‑polled).
  - `PUT /v1/enroll/{code}/approve` — Approve in Web (name/description optional).

- Admin
  - `DELETE /v1/admin/cleanup-expired` — Trigger cleanup and display summary.

### Error Handling

- **Network Errors**: Automatic retry with exponential backoff
- **Authentication Errors**: Automatic logout and redirect
- **Validation Errors**: Form-level error display
- **API Errors**: User-friendly error messages

## Development Workflow

### Adding New Features

1. **API Integration**: Add new endpoints to `src/lib/api.ts`
2. **Type Definitions**: Update `src/lib/types.ts`
3. **Hooks**: Create React Query hooks in `src/hooks/use-api.ts`
4. **Components**: Build UI components using shadcn/ui
5. **Routes**: Add new pages in `src/routes/`
6. **State**: Extend stores if needed

### Code Style

- **TypeScript**: Strict mode enabled
- **ESLint**: Code quality rules
- **Prettier**: Code formatting
- **Husky**: Pre-commit hooks

## Security Considerations

- **Token Storage**: Secure token handling in localStorage
- **CORS**: Properly configured for development/production
- **Validation**: Input validation on both client and server
- **HTTPS**: Use HTTPS in production environments

## Deployment

### Production Build

```bash
# Build optimized production bundle
pnpm run build

# Serve static files
pnpm run preview
```

### Docker Deployment

```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
RUN npm run build
EXPOSE 3000
CMD ["npm", "run", "preview"]
```

## Troubleshooting

### Common Issues

1. **CORS Errors**: Ensure sealbox-server has CORS enabled
2. **Login Failures**: Verify server URL and token are correct
3. **Build Errors**: Check Node.js version compatibility
4. **Network Issues**: Verify server connectivity

### Debug Mode

Enable React Query Devtools for debugging:
- Development mode: Devtools available in browser
- Inspect network requests and cache state
- Monitor authentication status

## Future Enhancements (Admin‑first)

- Permission templates and policy presets for common client groups.
- Bulk actions from the matrix view (multi‑revoke, status updates).
- Improved audit filtering and export.
- Team/RBAC for shared administration.
- CLI command generator for guided remediation (rotate/revoke/cleanup).

These focus on visual administration that complements CLI automation.
