// 与 sealbox-server API 对应的类型定义

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

// API 请求/响应类型
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

// 错误响应类型
export interface ApiError {
  error: string;
  message?: string;
}

// 配置类型
export interface AppConfig {
  serverUrl: string;
  token?: string;
}

// 认证状态类型
export interface AuthState {
  isAuthenticated: boolean;
  token?: string;
  serverUrl?: string;
}