# Sealbox Multi-Client Architecture Design

## Design Discussion Records

### Problem Discovery and Analysis

#### Current Architecture Limitations

After analyzing Sealbox's existing Client Key lifecycle, we identified an important architectural issue: **scalability problems of the current "by user" model in multi-user environments**.

**Current Architecture Characteristics**:
- Each user owns one Client Key pair (public/private key)
- A secret can only be decrypted by one client key
- Multi-user access requires sharing private keys, violating security principles

**Identified Problems**:
1. **Sharing Difficulties**: How to securely share secrets during team collaboration?
2. **Coarse-grained Permissions**: Unable to implement fine-grained access control
3. **Security Risks**: Sharing private keys violates cryptographic best practices
4. **Poor Scalability**: Difficult to adapt to complex enterprise environments

### Solution Exploration

#### "by client" Architecture Philosophy

After deep consideration, we proposed an important architectural improvement: **Client Key should be by client rather than by user**.

**Core Concept**:
- Each `sealbox-cli` instance has its own independent Client Key
- Each secret can be authorized to multiple clients
- Choose which clients can access when creating secrets
- No support for subsequent dynamic permission addition (server doesn't store plaintext)

**Design Principles**:
1. **Zero Trust Security Model**: Each CLI instance is an independent security principal
2. **Fine-grained Access Control**: Precise control over which clients can access which secrets
3. **Immutable Permissions**: Permissions determined at creation time, avoiding subsequent tampering
4. **Explicit Authorization**: Must explicitly declare who can access what

#### In-depth Analysis of DataKey Relationships

During the design process, we focused on discussing **DataKey relationships in multi-client architecture**, which is the core of the entire solution:

**Key Design Decision: One Secret = One DataKey**

```
Secret("database-password") 
├── DataKey: random_256_bit_key
├── encrypted_data: AES(DataKey, "mysqlpass123")  [only one copy stored]
└── Multiple encrypted_data_key records:
    ├── RSA(client_A_pubkey, DataKey) 
    ├── RSA(client_B_pubkey, DataKey)
    └── RSA(client_C_pubkey, DataKey)
```

**Why Share DataKey?**
1. **Data Consistency**: All authorized clients see the same plaintext after decryption
2. **Storage Efficiency**: encrypted_data stored only once, saving space
3. **Cryptographic Correctness**: Follows Envelope Encryption design principles

**Security Verification**:
- ✅ Each client can only decrypt with their own private key
- ✅ Clients without private keys cannot access
- ✅ Server never knows DataKey or plaintext  
- ✅ Revoke access: delete corresponding encrypted_data_key record

### Threat Model Analysis

We thoroughly analyzed the security properties of the new architecture:

**Core Security Properties**:
1. **Confidentiality**: Server never touches plaintext and DataKey
2. **Integrity**: AES-GCM and RSA-OAEP provide authenticated encryption
3. **Access Control**: Explicit authorization at creation time, supports precise revocation

**Threat Model Coverage**:
- ✅ Server compromise: Attackers can only see encrypted data
- ✅ Network eavesdropping: Only encrypted data in transit  
- ✅ Insider threats: Administrators cannot peek at plaintext
- ✅ Client compromise: Single client breach doesn't affect other clients
- ✅ Permission creep: No dynamic permission addition, avoiding accidental authorization

## Formal Design Proposal

### Architecture Overview

#### Evolution from "by user" to "by client"

**Current Architecture (by user)**:
```
User ——— Client Key ——— Multiple Secrets
  │          │               │
  └─ Private Key └─ Public Key └─ Encrypted Data
```

**New Architecture (by client)**:
```
User ——— Multiple Clients ——— Shared Secrets Pool
  │          │                      │
  └─ Management └─ Independent Key Pairs └─ Multi-encryption
```

#### Core Components

1. **Client Registry**: Manages all registered clients
2. **Multi-Key Secret Storage**: Secret storage supporting multi-client encryption
3. **Authorization Matrix**: Access relationships between clients and secrets
4. **Key Rotation System**: Supports client key rotation

### Database Design

#### New Table Structure

```sql
-- Client registry table
CREATE TABLE clients (
    id BLOB PRIMARY KEY,                -- UUID, unique client identifier
    name TEXT NOT NULL,                 -- Client name/alias
    public_key TEXT NOT NULL,           -- Client public key (PEM format)
    description TEXT,                   -- Optional description
    created_at INTEGER NOT NULL,        -- Registration time
    last_used_at INTEGER,               -- Last used time
    status TEXT NOT NULL DEFAULT 'Active', -- Active/Disabled/Retired
    metadata TEXT                       -- Optional metadata (JSON)
);

-- Secret-client key association table
CREATE TABLE secret_client_keys (
    secret_key TEXT NOT NULL,           -- Secret identifier
    secret_version INTEGER NOT NULL,    -- Secret version
    client_id BLOB NOT NULL,            -- Client ID (references clients.id)
    encrypted_data_key BLOB NOT NULL,   -- DataKey encrypted with this client's public key
    created_at INTEGER NOT NULL,        -- Authorization time
    
    PRIMARY KEY (secret_key, secret_version, client_id),
    FOREIGN KEY (client_id) REFERENCES clients(id),
    FOREIGN KEY (secret_key, secret_version) REFERENCES secrets(key, version)
);

-- Index optimization for existing secrets table
CREATE INDEX idx_secrets_key_version ON secrets(key, version);
CREATE INDEX idx_secret_client_keys_client ON secret_client_keys(client_id);
CREATE INDEX idx_secret_client_keys_secret ON secret_client_keys(secret_key, secret_version);
```

#### Data Migration Strategy

**Phase One: Compatibility Migration**
1. Create new table structures
2. Keep existing client_keys table unchanged
3. Create default client records for existing secrets
4. Migrate existing encrypted_data_key to secret_client_keys table

**Phase Two: Complete Migration**  
1. All new features use new table structure
2. Provide migration tools to convert old data to new format
3. Deprecate old tables after confirming stability

### API Design

#### Client Management API

```http
# Register new client
POST /v1/clients
Content-Type: application/json
Authorization: Bearer <token>

{
    "name": "morris-laptop",
    "public_key": "-----BEGIN RSA PUBLIC KEY-----\n...",
    "description": "Morris's development laptop"
}

Response:
{
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "morris-laptop", 
    "created_at": 1640995200,
    "status": "Active"
}
```

```http
# List all clients
GET /v1/clients
Authorization: Bearer <token>

Response:
{
    "clients": [
        {
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "morris-laptop",
            "description": "Morris's development laptop",
            "created_at": 1640995200,
            "last_used_at": 1640995800,
            "status": "Active"
        }
    ]
}
```

```http
# Disable/enable client
PUT /v1/clients/{client_id}/status
Content-Type: application/json
Authorization: Bearer <token>

{
    "status": "Disabled"  // Active/Disabled/Retired
}
```

#### Multi-Client Secret Management API

```http
# Create multi-client secret
PUT /v1/secrets/{key}
Content-Type: application/json
Authorization: Bearer <token>

{
    "secret": "mysqlpass123",
    "authorized_clients": [
        "550e8400-e29b-41d4-a716-446655440000",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    ],
    "ttl": 3600,
    "description": "Production database password"
}

Response:
{
    "key": "prod-db-password",
    "version": 1,
    "authorized_clients": 2,
    "created_at": 1640995200,
    "expires_at": 1640998800
}
```

```http
# Get secret (automatically detect calling client)
GET /v1/secrets/{key}?version={version}
Authorization: Bearer <token>
X-Client-ID: 550e8400-e29b-41d4-a716-446655440000

Response:
{
    "key": "prod-db-password",
    "version": 1,
    "encrypted_data": [/* AES encrypted data */],
    "encrypted_data_key": [/* RSA encrypted DataKey for this client */],
    "created_at": 1640995200,
    "expires_at": 1640998800
}
```

```http
# View secret access permissions
GET /v1/secrets/{key}/permissions
Authorization: Bearer <token>

Response:
{
    "key": "prod-db-password",
    "authorized_clients": [
        {
            "client_id": "550e8400-e29b-41d4-a716-446655440000",
            "client_name": "morris-laptop",
            "authorized_at": 1640995200
        },
        {
            "client_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", 
            "client_name": "ci-server",
            "authorized_at": 1640995200
        }
    ]
}
```

```http
# Revoke client access permission
DELETE /v1/secrets/{key}/permissions/{client_id}
Authorization: Bearer <token>

Response: 204 No Content
```

### CLI Design Changes

#### New Client Management Commands

```bash
# Client registration
sealbox-cli client register --name "morris-laptop" --description "Development laptop"

# List clients
sealbox-cli client list

# Disable client
sealbox-cli client disable <client-id>

# Check client status
sealbox-cli client status
```

#### Enhanced Secret Management Commands

```bash
# Create multi-client secret
sealbox-cli secret set prod-db-password \
    --clients client-id-1,client-id-2 \
    --ttl 3600

# View secret permissions
sealbox-cli secret permissions prod-db-password

# Revoke client permissions  
sealbox-cli secret revoke prod-db-password --client client-id-1
```

### Web UI Enhancements

#### Client Management Interface

1. **Client List Page**
   - Display all registered clients
   - Client status (Active/Disabled/Retired)  
   - Last used time
   - Enable/disable operations

2. **Client Details Page**
   - Client basic information
   - List of secrets this client has access to
   - Usage statistics and history

#### Secret Permission Visualization

1. **Secret Creation Page**
   - Client selector (multi-select)
   - Permission preview
   - Bulk authorization functionality

2. **Permission Management Page**
   - Permission matrix view (Secrets × Clients)
   - Bulk permission operations
   - Permission change history

### Implementation Plan

#### Phase One: Infrastructure Setup (2-3 weeks)

**Objective**: Establish the foundation for multi-client architecture

**Tasks**:
1. **Database Migration**
   - [ ] Create new table structures
   - [ ] Write migration scripts
   - [ ] Backward compatibility testing

2. **Core API Development**
   - [ ] Client registration API
   - [ ] Client management API  
   - [ ] Basic multi-client secret creation API

3. **CLI Basic Functionality**
   - [ ] Client registration commands
   - [ ] Multi-client secret creation commands

**Acceptance Criteria**:
- Ability to register multiple clients
- Ability to create multi-client secrets
- Existing functionality remains compatible

#### Phase Two: Complete Feature Implementation (3-4 weeks)

**Objective**: Implement complete multi-client functionality

**Tasks**:
1. **Permission Management**
   - [ ] Secret permission query API
   - [ ] Permission revocation API
   - [ ] Client status management

2. **Complete CLI Functionality**
   - [ ] Permission viewing commands
   - [ ] Permission revocation commands
   - [ ] Client status management commands

3. **Security Enhancements**
   - [ ] Client authentication
   - [ ] Access logging
   - [ ] Security audit features

**Acceptance Criteria**:
- Complete permission management functionality
- Secure client authentication
- Detailed operation auditing

#### Phase Three: Web UI and Optimization (2-3 weeks)

**Objective**: Provide complete user interface and performance optimization

**Tasks**:
1. **Web UI Development**
   - [ ] Client management interface
   - [ ] Permission visualization interface
   - [ ] Permission matrix view

2. **Performance Optimization**
   - [ ] Database query optimization
   - [ ] Bulk operation optimization
   - [ ] Caching mechanisms

3. **Documentation and Testing**
   - [ ] API documentation updates
   - [ ] User manual updates
   - [ ] Integration testing improvements

**Acceptance Criteria**:
- Intuitive web management interface
- Good performance characteristics
- Complete documentation and testing

### Use Cases

#### Team Collaboration Scenario

**Scenario**: Development team needs to share database passwords

```bash
# DevOps engineer creates database password
sealbox-cli secret set prod-db-password "complex-password" \
    --clients devops-laptop,ci-server,prod-deployment-server

# Developer uses on CI server
sealbox-cli secret get prod-db-password
```

**Advantages**:
- Each environment uses independent clients
- Can individually revoke access for specific environments
- No need to share private keys

#### Service Deployment Scenario

**Scenario**: Microservices need access to shared API keys

```bash
# Create API key for multiple microservice instances
sealbox-cli secret set api-key "sk-..." \
    --clients service-a-prod,service-b-prod,service-c-prod

# If a service has security issues, revoke individually
sealbox-cli secret revoke api-key --client service-b-prod
```

#### Permission Audit Scenario

**Scenario**: Regular review of access permissions

```bash
# View permission distribution for all secrets
sealbox-cli secret list --show-permissions

# View access scope for specific client
sealbox-cli client permissions <client-id>

# View permission change history
sealbox-cli audit permissions --since 30d
```

### Backward Compatibility

#### Migration Strategy

1. **Progressive Migration**
   - Existing APIs continue to work
   - New features use new APIs
   - Provide automatic migration tools

2. **Compatibility APIs**
   - Old version CLI continues to work
   - New version CLI supports both modes
   - Clear deprecation timeline

3. **Data Migration**
   - Existing secrets automatically create corresponding client records
   - Keep existing encrypted data unchanged
   - Provide rollback mechanisms

### Performance Considerations

#### Storage Optimization

**Analysis**:
- `encrypted_data`: One copy per secret, storage unchanged
- `encrypted_data_key`: One copy per client, increased storage
- **Estimation**: N secrets × M clients = N×M records

**Optimization Strategies**:
1. `encrypted_data_key` is small (256 bytes), storage overhead is manageable
2. Reasonable index design improves query performance  
3. Periodic cleanup of permission records for disabled clients

#### Query Optimization

**High-frequency Queries**:
1. Query accessible secrets by client ID
2. Query authorized clients by secret
3. Verify specific client access to specific secret

**Index Strategy**:
```sql
-- Support clients querying their accessible secrets
CREATE INDEX idx_secret_client_keys_client_key 
ON secret_client_keys(client_id, secret_key);

-- Support secrets querying their authorized clients  
CREATE INDEX idx_secret_client_keys_secret_client
ON secret_client_keys(secret_key, secret_version, client_id);

-- Support time-based permission auditing
CREATE INDEX idx_secret_client_keys_created_at
ON secret_client_keys(created_at);
```

### Security Enhancements

#### Client Authentication

**Current Approach**: Bearer Token verification
**Enhanced Approach**: Add client certificate verification

```http
# API calls require both
Authorization: Bearer <server-token>
X-Client-ID: <client-uuid>  
X-Client-Signature: <signature-of-request>
```

#### Access Auditing

**Audit Records**:
```sql
CREATE TABLE access_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id BLOB NOT NULL,
    secret_key TEXT NOT NULL,
    action TEXT NOT NULL,        -- get/create/delete/revoke
    timestamp INTEGER NOT NULL,
    source_ip TEXT,
    user_agent TEXT,
    success BOOLEAN NOT NULL
);
```

#### Enhanced Key Rotation

**Client Key Rotation**:
1. Generate new client key pair
2. Re-encrypt all DataKeys with new public key
3. Update secret_client_keys table
4. Atomic operation ensuring consistency

## Summary and Next Steps

### Key Design Decisions Summary

1. **"by client" rather than "by user"**: Each CLI instance has independent security identity
2. **Shared DataKey design**: Balances security, consistency, and storage efficiency
3. **Authorization at creation time**: No dynamic permission expansion, enhancing security
4. **Progressive migration**: Ensures backward compatibility, reduces migration risk

### Implementation Priorities

**P0 (Must Implement)**:
- New database table structures
- Client registration API
- Multi-client secret creation API
- Basic CLI commands

**P1 (Important Features)**:
- Permission management API
- Permission revocation functionality
- Basic Web UI interface
- Access auditing

**P2 (Enhancement Features)**:
- Advanced permission visualization
- Performance optimization
- Client certificate verification
- Detailed audit reports

### Future Extension Considerations

1. **Organizational Structure Support**: Support for team and department-level permissions
2. **Temporary Access**: Support for time-limited permission grants
3. **Permission Templates**: Pre-defined common permission combinations
4. **Integration Support**: Integration with LDAP, OAuth, and other identity systems
5. **High Availability Deployment**: Support for multi-node deployment permission synchronization

### Risk Assessment

**Technical Risks**:
- Data migration complexity: Mitigated through progressive migration
- Performance impact: Verified through proper indexing and testing
- Compatibility issues: Keep old APIs working

**Security Risks**:
- Permission complexity: Mitigated through clear UI and auditing
- Client proliferation: Controlled through status management and periodic cleanup
- Operational error risk: Reduced through confirmation mechanisms and operation logs

This multi-client architecture proposal has undergone thorough security analysis and practical considerations. It addresses the limitations of the existing architecture while laying the foundation for future expansion. Through phased implementation, new features can be gradually introduced while ensuring stability.

## Implementation Status (2025-08-24)

### ✅ Completed Features

**Phase 1: Infrastructure Setup (Completed)**
- ✅ Database migrations and new table structures
- ✅ Core API development (Client management, Multi-client secret creation)
- ✅ CLI basic functionality (Client operations, Multi-client secret commands)
- ✅ Backward compatibility maintained

**Phase 2: Complete Feature Implementation (Completed)**
- ✅ Permission management APIs (view, revoke permissions)
- ✅ Complete CLI functionality (permissions, revocation commands)  
- ✅ Security enhancements (client authentication, access validation)
- ✅ Comprehensive testing (77 test cases covering multi-client scenarios)

**Phase 3: Web UI and Optimization (Completed)**
- ✅ Web UI development (client management, permission visualization)
- ✅ Multi-client secret creation interface
- ✅ Permission management UI with revocation capabilities
- ✅ Complete internationalization (English, Chinese, Japanese, German)
- ✅ TypeScript type safety and modern React architecture

### 🚀 Complete Usage Examples

#### Basic Multi-Client Secret Management

```bash
# 1. Generate and register client keys for different environments
# Development laptop
sealbox-cli key generate --force
sealbox-cli key register

# Get client ID for development laptop  
DEV_CLIENT_ID=$(sealbox-cli key list --output json | jq -r '.client_keys[0].id')

# Production server (on production machine)
sealbox-cli key generate --force  
sealbox-cli key register
PROD_CLIENT_ID=$(sealbox-cli key list --output json | jq -r '.client_keys[0].id')

# CI/CD pipeline (on CI machine)
sealbox-cli key generate --force
sealbox-cli key register  
CI_CLIENT_ID=$(sealbox-cli key list --output json | jq -r '.client_keys[0].id')

# 2. Create multi-client secrets
# Database password accessible by all environments
sealbox-cli secret set-multi-client database-password \
    --clients "$DEV_CLIENT_ID,$PROD_CLIENT_ID,$CI_CLIENT_ID" \
    --ttl 86400

# API key only for production and CI
sealbox-cli secret set-multi-client api-key \
    --clients "$PROD_CLIENT_ID,$CI_CLIENT_ID" \
    --ttl 3600

# Development-only secret
sealbox-cli secret set dev-debug-token "debug-12345" --ttl 7200
```

#### Permission Management

```bash  
# View permissions for a secret
sealbox-cli secret permissions database-password

# Sample output:
# {
#   "key": "database-password",
#   "authorized_clients": [
#     {
#       "client_id": "550e8400-e29b-41d4-a716-446655440000",
#       "authorized_at": 1692889200
#     },
#     {
#       "client_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", 
#       "authorized_at": 1692889200
#     }
#   ]
# }

# Revoke access for specific client
sealbox-cli secret revoke database-password --client "$DEV_CLIENT_ID"

# Verify revocation
sealbox-cli secret permissions database-password
```

#### Team Collaboration Workflow

```bash
# Team lead creates shared secrets
sealbox-cli secret set-multi-client shared-db-connection \
    --clients "$ALICE_CLIENT,$BOB_CLIENT,$CHARLIE_CLIENT" \
    "postgresql://user:pass@db.company.com/app"

# Each team member can access the secret independently  
# Alice's machine:
sealbox-cli secret get shared-db-connection
# Output: postgresql://user:pass@db.company.com/app

# Bob's machine:  
sealbox-cli secret get shared-db-connection
# Output: postgresql://user:pass@db.company.com/app

# If Bob leaves the team, revoke his access
sealbox-cli secret revoke shared-db-connection --client "$BOB_CLIENT"

# Bob can no longer access the secret, Alice and Charlie can still access it
```

#### Web UI Multi-Client Management

The Web UI now provides complete multi-client support:

1. **Multi-Client Secret Creation**: 
   - Toggle "Multi-Client Mode" in the create secret dialog
   - Select multiple active client keys from the list
   - Visual client badges show selected clients

2. **Permission Management**:
   - Click the shield (🛡️) icon next to any secret in the table/card view  
   - View all authorized clients with timestamps
   - Revoke individual client permissions with confirmation dialog
   - Real-time permission updates

3. **Client Key Management**:
   - View all registered client keys with status
   - CLI integration guide for key operations
   - Status indicators (Active/Disabled/Retired)

### 🔒 Security Properties Verified

- ✅ **True Shared DataKey Design**: One secret = one DataKey, encrypted separately for each authorized client
- ✅ **Zero Server Knowledge**: Server never sees plaintext data or DataKeys
- ✅ **Client Isolation**: Compromise of one client doesn't affect others  
- ✅ **Permission Immutability**: Permissions set at creation time, preventing unauthorized expansion
- ✅ **Cryptographic Integrity**: RSA-2048 + AES-256-GCM with authenticated encryption

### 📊 Test Coverage

- **77 total test cases** across CLI and server components
- **Multi-client specific tests**: 15+ test cases covering creation, retrieval, permission management
- **Backward compatibility**: All existing single-client functionality preserved
- **Error handling**: Comprehensive validation and error scenarios covered
- **Security validation**: Client key validation, authorization checks, permission boundaries

### 🌐 Internationalization

Complete multi-language support for all new features:
- **English** (default)
- **中文** (Chinese Simplified) 
- **日本語** (Japanese)
- **Deutsch** (German)

All UI text, error messages, and help text fully translated.

This implementation represents a complete, production-ready multi-client architecture that maintains backward compatibility while providing powerful new capabilities for team collaboration and fine-grained access control.