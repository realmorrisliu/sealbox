// Type definitions corresponding to sealbox-server API

export interface SecretInfo {
  key: string;
  version: number;
  created_at: number;
  updated_at: number;
  expires_at?: number;
}

export interface Secret {
  namespace: string;
  key: string;
  version: number;
  encrypted_data: number[];
  encrypted_data_key: number[];
  master_key_id: string;
  created_at: number;
  updated_at: number;
  expires_at?: number;
  metadata?: string;
}

export interface MasterKey {
  id: string;
  public_key: string;
  created_at: number;
  status: "Active" | "Retired" | "Disabled";
  description?: string;
  metadata?: string;
}

// API request/response types
export interface SecretsListResponse {
  secrets: SecretInfo[];
}

export interface CreateSecretRequest {
  secret: string;
  ttl?: number;
}

export interface CreateMasterKeyRequest {
  public_key: string;
}

export interface MasterKeysListResponse {
  master_keys: MasterKey[];
}

export interface CleanupExpiredResponse {
  deleted_count: number;
  cleaned_at: number;
}

// Error response types
export interface ApiError {
  error: string;
  message?: string;
}

// UI-specific types for enhanced secret display
export type SecretStatus = "active" | "expiring" | "expired";
export type RiskLevel = "low" | "medium" | "high" | "critical";
export type Environment = "development" | "staging" | "production";
export type SecretCategory = "password" | "api-key" | "certificate" | "token" | "other";

export interface SecretVersion {
  id: string;
  version: number;
  value: string;
  changedBy: string;
  changedAt: string;
  changeReason: string;
  isCurrent: boolean;
  environment: Environment;
  riskLevel: RiskLevel;
  tags: string[];
}

export interface SecretUIData {
  id: string;
  name: string;
  value: string;
  description?: string;
  environment: Environment;
  category: SecretCategory;
  tags: string[];
  status: SecretStatus;
  riskLevel: RiskLevel;
  createdAt: string;
  lastUsed?: string;
  lastModified: string;
  lastRotated?: string;
  expiresAt?: string;
  isFavorite: boolean;
  isArchived: boolean;
  accessCount: number;
  versions: SecretVersion[];
  // API fields for reference
  key: string;
  version: number;
  created_at: number;
  updated_at: number;
  expires_at?: number;
}

// Utility function to convert API SecretInfo to UI SecretUIData
export function convertSecretToUIData(secret: SecretInfo): SecretUIData {
  const now = Date.now() / 1000;
  const isExpired = secret.expires_at && now > secret.expires_at;
  const isExpiring = secret.expires_at && !isExpired && (secret.expires_at - now) < 7 * 24 * 3600; // Expiring in 7 days
  
  return {
    // UI display fields
    id: `${secret.key}-${secret.version}`,
    name: secret.key,
    value: "[ENCRYPTED]", // Server-side encrypted data
    description: undefined,
    environment: "production" as const, // Could be inferred from key naming convention
    category: "other" as const,
    tags: [],
    status: isExpired ? "expired" : isExpiring ? "expiring" : "active",
    riskLevel: "medium" as const,
    createdAt: new Date(secret.created_at * 1000).toISOString().split("T")[0],
    lastUsed: undefined, // Not available from server API
    lastModified: new Date(secret.updated_at * 1000).toISOString().split("T")[0],
    lastRotated: new Date(secret.created_at * 1000).toISOString().split("T")[0],
    expiresAt: secret.expires_at ? new Date(secret.expires_at * 1000).toISOString().split("T")[0] : undefined,
    isFavorite: false,
    isArchived: false,
    accessCount: 0, // Not available from server API
    versions: [{
      id: `v${secret.key}-${secret.version}`,
      version: secret.version,
      value: "[ENCRYPTED]",
      changedBy: "system", // Not available from server API
      changedAt: new Date(secret.created_at * 1000).toISOString().replace("T", " ").substring(0, 19),
      changeReason: "Current version",
      isCurrent: true,
      environment: "production",
      riskLevel: "medium",
      tags: []
    }],
    // Original API fields for reference
    key: secret.key,
    version: secret.version,
    created_at: secret.created_at,
    updated_at: secret.updated_at,
    expires_at: secret.expires_at
  };
}

// Configuration types
export interface AppConfig {
  serverUrl: string;
  token?: string;
}

// Authentication state types
export interface AuthState {
  isAuthenticated: boolean;
  token?: string;
  serverUrl?: string;
}