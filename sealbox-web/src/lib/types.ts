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

// Health check response types
export interface HealthResponse {
  result: string;
  timestamp: number;
}

// UI display status for secrets
export type SecretStatus = "active" | "expiring" | "expired";

// Simplified UI data matching server capabilities
export interface SecretUIData {
  key: string;
  version: number;
  created_at: number;
  updated_at: number;
  expires_at?: number;
  // Computed display fields
  status: SecretStatus;
  createdAt: string;
  updatedAt: string;
  expiresAt?: string;
}

// Utility function to convert API SecretInfo to UI display data
export function convertSecretToUIData(secret: SecretInfo): SecretUIData {
  const now = Date.now() / 1000;
  const isExpired = secret.expires_at && now > secret.expires_at;
  const isExpiring = secret.expires_at && !isExpired && (secret.expires_at - now) < 7 * 24 * 3600; // Expiring in 7 days
  
  return {
    // Original API fields
    key: secret.key,
    version: secret.version,
    created_at: secret.created_at,
    updated_at: secret.updated_at,
    expires_at: secret.expires_at,
    // Computed display fields
    status: isExpired ? "expired" : isExpiring ? "expiring" : "active",
    createdAt: new Date(secret.created_at * 1000).toISOString().split("T")[0],
    updatedAt: new Date(secret.updated_at * 1000).toISOString().split("T")[0],
    expiresAt: secret.expires_at ? new Date(secret.expires_at * 1000).toISOString().split("T")[0] : undefined
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