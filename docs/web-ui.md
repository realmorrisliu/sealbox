# Sealbox Web UI

Sealbox Web UI is a modern React-based web interface for managing secrets through an intuitive browser interface.

## Features

- 🔐 **Secure Authentication** - Bearer Token authentication with session persistence
- 📋 **Secret Management** - View, delete secrets with real-time status updates
- ⏰ **TTL Indicators** - Visual countdown and expiration warnings
- 📱 **Responsive Design** - Works seamlessly on desktop and mobile devices
- 🌐 **CORS Support** - Development-friendly cross-origin request handling
- 🎨 **Modern UI** - Built with TailwindCSS and shadcn/ui components
- 🌍 **English-First Interface** - Clean, professional English text throughout
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
4. **Experience the English interface** - All text is clear and professional

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
│   ├── hooks/
│   │   └── use-api.ts              # API integration hooks
│   ├── lib/
│   │   ├── api.ts                  # API client
│   │   ├── types.ts                # TypeScript definitions
│   │   ├── utils.ts                # Utilities
│   │   └── query-client.ts         # React Query config
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

- **Responsive**: Adapts to different screen sizes
- **Dark Mode**: Supports light/dark theme preferences
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
- `GET /v1/master-key` - List master keys
- `POST /v1/master-key` - Register master key

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

## Future Enhancements

- [ ] **i18n Support** - Multi-language interface with language switching
- [ ] Secret creation and editing interface
- [ ] Master key management UI
- [ ] Bulk secret operations
- [ ] Export/import functionality
- [ ] Advanced search and filtering
- [ ] User preference settings
- [ ] Theme customization (dark/light modes)