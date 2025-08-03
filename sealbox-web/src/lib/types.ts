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