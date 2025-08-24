# Sealbox Web UI - Visual Secret Management

Sealbox Web UI is a modern React-based web interface **optimized for visual secret management and administration**. It complements the CLI by providing intuitive tools for oversight, bulk operations, and team collaboration.

## Features - Management-Focused Design

### 🎯 Core Philosophy: Visual Administration
Web UI focuses on **management tasks** that benefit from visual oversight, while CLI handles automation and consumption.

- 📊 **Secret Overview Dashboard** - Visual statistics, search, and filtering for all secrets
- 🗂️ **Batch Operations** - Multi-select create/delete operations for efficiency
- ⏰ **TTL Management** - Real-time countdown timers and expiration warnings
- 👥 **Team Collaboration** - Multi-language interface for diverse teams
- 📋 **Visual Secret Lifecycle** - Create, view, update, delete with visual confirmation

### 🔧 Technical Features
- 🔐 **Secure Authentication** - Bearer Token authentication with session persistence
- 📱 **Responsive Design** - Works seamlessly on desktop and mobile devices
- 🌐 **CORS Support** - Development-friendly cross-origin request handling
- 🎨 **Modern Industrial UI** - 2025 design following Linear/Superhuman principles with strict 8pt grid system
- 🌍 **Internationalization** - Support for English, Chinese, Japanese, and German
- 🚀 **Production Ready** - Optimized builds and proper error handling

## Technology Stack

- **Frontend Framework**: React 19 + TanStack Start
- **Routing**: TanStack Router (file-based)
- **State Management**: Zustand with persistence
- **Data Fetching**: TanStack Query with caching
- **Forms**: React Hook Form + Zod validation
- **Styling**: TailwindCSS + shadcn/ui
- **Build Tool**: Vite
- **Language**: TypeScript
- **Internationalization**: react-i18next + i18next-browser-languagedetector

## Web UI vs CLI: Complementary Tools

### 🌐 Web UI: Visual Management & Administration
**Best for:**
- Daily secret management with visual oversight
- Creating and organizing secrets with forms and validations
- Monitoring secret status and TTL across your entire inventory
- Team collaboration with multi-language support
- Batch operations (multi-select create/delete)
- Understanding secret usage patterns through statistics

### 🖥️ CLI: Automation & Secret Consumption  
**Best for:**
- CI/CD pipelines and automated deployments
- Exporting secrets as environment variables
- Scripting and programmatic access
- One-time key generation and setup
- Bulk import from configuration files

**Example Workflows:**
```bash
# CLI: Export secrets for production deployment
sealbox-cli secret export --format shell --prefix PROD > prod.env
source prod.env

# Web UI: Create new secrets with TTL using visual forms
# Web UI: Monitor which secrets are expiring soon
# Web UI: Bulk delete old development secrets
```

This separation ensures you have the **right tool for each task** - visual interface for human oversight, CLI for automation.

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

### Authentication System

- **Token-based**: Uses Bearer token authentication
- **Persistent**: Login state persists across browser sessions
- **Secure**: Tokens stored in localStorage with proper validation
- **Connection Testing**: Validates server connectivity during login

### Secret Management

- **List View**: Displays all secrets with metadata
- **TTL Status**: Visual indicators for expiration status:
  - 🟢 Normal: More than 24 hours until expiration
  - 🟡 Warning: Less than 24 hours until expiration
  - 🔴 Critical: Less than 1 hour until expiration
- **Real-time Updates**: Automatic refresh and status updates
- **Deletion**: Secure deletion with confirmation dialogs

### User Interface

- **2025 Modern Industrial Design**: Following Linear/Superhuman style principles
  - **8pt Grid System**: Strict spacing hierarchy (64px→32px→16px→8px)
  - **Function-First Colors**: Minimal gradient usage, strong grayscale hierarchy
  - **3-Layer Architecture**: Clear page information structure (Header→Content→Footer)
  - **Visual Restraint**: Clean typography with Inter font and optimized tracking
  - **Consistent Interactions**: 150ms transition duration standard
- **Responsive**: Adapts to different screen sizes with mobile-first approach
- **Multi-language**: Support for 4 languages with smart switching
- **Language Persistence**: Remembers user's language preference
- **Localized Dates**: Date formatting according to user's language
- **Accessible**: WCAG compliant components
- **Fast**: Optimized loading and caching

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

The Web UI integrates with all sealbox-server APIs:

- `GET /v1/secrets` - List secrets
- `GET /v1/secrets/:key` - Get secret details
- `DELETE /v1/secrets/:key` - Delete secret
- `GET /v1/client-key` - List client keys
- `POST /v1/client-key` - Register client key

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

## Future Enhancements - Management Focus

### ✅ Completed Features
- [x] **i18n Support** - Multi-language interface with language switching
- [x] **Industrial UI Design** - 2025 modern design system with Linear/Superhuman principles
- [x] **Secret Overview Dashboard** - Visual statistics and TTL monitoring
- [x] **Authentication System** - Token-based auth with persistence

### 🚧 Planned Management Features
- [ ] **Enhanced Secret Creation** - Rich forms with validation and preview
- [ ] **Advanced Batch Operations** - Multi-select with bulk actions toolbar
- [ ] **Secret Categories & Tags** - Organization and filtering system
- [ ] **Usage Analytics** - Access patterns and secret lifecycle insights
- [ ] **Team Management** - User roles and collaboration features
- [ ] **Audit Trail** - Visual history of secret changes and access
- [ ] **Advanced Search** - Full-text search across secret metadata
- [ ] **Export Wizard** - Guided export with format preview
- [ ] **Permission Templates** - Reusable permission sets for new secrets
- [ ] **Dashboard Customization** - Personalized views and widgets

### 🔗 CLI Integration Features
- [ ] **CLI Command Generator** - Visual tool to generate CLI commands
- [ ] **Key Status Monitoring** - Visual display of CLI-generated key pairs
- [ ] **Import Status Tracking** - Monitor CLI bulk import progress

These enhancements focus on **visual management capabilities** that complement CLI's automation strengths.
