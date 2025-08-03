import type {
  SecretInfo,
  Secret,
  MasterKey,
  SecretsListResponse,
  CreateSecretRequest,
  CreateMasterKeyRequest,
  MasterKeysListResponse,
  CleanupExpiredResponse,
  ApiError,
} from "./types";

export class SealboxApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public body?: any,
  ) {
    super(message);
    this.name = "SealboxApiError";
  }
}

export class SealboxApi {
  private baseUrl: string;
  private token?: string;

  constructor(baseUrl: string, token?: string) {
    this.baseUrl = baseUrl.replace(/\/$/, ""); // Remove trailing slash
    this.token = token;
  }

  setToken(token: string) {
    this.token = token;
  }

  setBaseUrl(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    
    const headers = new Headers({
      "Content-Type": "application/json",
      ...options.headers,
    });

    if (this.token) {
      headers.set("Authorization", `Bearer ${this.token}`);
    }

    const response = await fetch(url, {
      ...options,
      headers,
    });

    if (!response.ok) {
      let errorBody;
      try {
        errorBody = await response.json();
      } catch {
        errorBody = { error: response.statusText };
      }
      
      throw new SealboxApiError(
        errorBody.error || errorBody.message || `HTTP ${response.status}`,
        response.status,
        errorBody,
      );
    }

    if (response.status === 204) {
      return {} as T;
    }

    return response.json();
  }

  // Secret management API
  async listSecrets(): Promise<SecretsListResponse> {
    return this.request<SecretsListResponse>("/v1/secrets");
  }

  async getSecret(key: string, version?: number): Promise<Secret> {
    const queryParam = version ? `?version=${version}` : "";
    return this.request<Secret>(`/v1/secrets/${encodeURIComponent(key)}${queryParam}`);
  }

  async createSecret(key: string, data: CreateSecretRequest): Promise<Secret> {
    return this.request<Secret>(`/v1/secrets/${encodeURIComponent(key)}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async deleteSecret(key: string, version: number): Promise<void> {
    return this.request<void>(`/v1/secrets/${encodeURIComponent(key)}?version=${version}`, {
      method: "DELETE",
    });
  }

  // Master key management API
  async listMasterKeys(): Promise<MasterKeysListResponse> {
    return this.request<MasterKeysListResponse>("/v1/master-key");
  }

  async createMasterKey(data: CreateMasterKeyRequest): Promise<void> {
    return this.request<void>("/v1/master-key", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async rotateMasterKey(data: CreateMasterKeyRequest): Promise<void> {
    return this.request<void>("/v1/master-key", {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  // Admin API
  async cleanupExpiredSecrets(): Promise<CleanupExpiredResponse> {
    return this.request<CleanupExpiredResponse>("/v1/admin/cleanup-expired", {
      method: "DELETE",
    });
  }

  // Health check
  async health(): Promise<{ status: string; service: string; timestamp: number }> {
    return this.request<{ status: string; service: string; timestamp: number }>("/healthz/live");
  }

  // Readiness check
  async readiness(): Promise<{ status: string; service: string; database: string; timestamp: number }> {
    return this.request<{ status: string; service: string; database: string; timestamp: number }>("/healthz/ready");
  }
}

// Create default instance
export const createApiClient = (baseUrl: string, token?: string) => {
  return new SealboxApi(baseUrl, token);
};

// Query keys for React Query
export const queryKeys = {
  secrets: ["secrets"] as const,
  secret: (key: string, version?: number) => ["secret", key, version] as const,
  masterKeys: ["masterKeys"] as const,
} as const;