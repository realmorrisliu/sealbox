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
  client_key_id: string;
  created_at: number;
  updated_at: number;
  expires_at?: number;
  metadata?: string;
}

export interface ClientKey {
  id: string;
  // server /v1/clients does not return public_key; keep optional to preserve compatibility
  public_key?: string;
  name?: string | null;
  description?: string | null;
  created_at: number;
  last_used_at?: number | null;
  status: "Active" | "Retired" | "Disabled";
  metadata?: string | null;
}

// API request/response types
export interface SecretsListResponse {
  secrets: SecretInfo[];
}

export interface SecretPermissionItem {
  client_id: string;
  client_name?: string | null;
  authorized_at: number;
}

export interface SecretPermissionsResponse {
  key: string;
  authorized_clients: SecretPermissionItem[];
}

export interface CreateSecretRequest {
  secret: string;
  ttl?: number;
}

export interface CreateClientKeyRequest {
  public_key: string;
}

export interface ClientKeysListResponse {
  client_keys: ClientKey[];
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
