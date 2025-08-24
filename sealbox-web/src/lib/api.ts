import type {
  SecretInfo,
  Secret,
  ClientKey,
  SecretsListResponse,
  CreateSecretRequest,
  CreateClientKeyRequest,
  ClientKeysListResponse,
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

  private async requestWithTiming<T>(
    endpoint: string,
    options: RequestInit = {},
  ): Promise<T & { responseTime: number }> {
    const startTime = performance.now();

    try {
      const result = await this.request<T>(endpoint, options);
      const endTime = performance.now();
      const responseTime = Math.round(endTime - startTime);

      return {
        ...result,
        responseTime,
      };
    } catch (error) {
      const endTime = performance.now();
      const responseTime = Math.round(endTime - startTime);

      if (error instanceof SealboxApiError) {
        // Add response time to error for potential debugging
        (error as any).responseTime = responseTime;
      }

      throw error;
    }
  }

  // Secret management API
  async listSecrets(): Promise<SecretsListResponse> {
    return this.request<SecretsListResponse>("/v1/secrets");
  }

  async getSecret(key: string, version?: number): Promise<Secret> {
    const queryParam = version ? `?version=${version}` : "";
    return this.request<Secret>(
      `/v1/secrets/${encodeURIComponent(key)}${queryParam}`,
    );
  }

  async createSecret(key: string, data: CreateSecretRequest): Promise<Secret> {
    return this.request<Secret>(`/v1/secrets/${encodeURIComponent(key)}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async deleteSecret(key: string, version: number): Promise<void> {
    return this.request<void>(
      `/v1/secrets/${encodeURIComponent(key)}?version=${version}`,
      {
        method: "DELETE",
      },
    );
  }


  // Client key management API
  async listClientKeys(): Promise<ClientKeysListResponse> {
    return this.request<ClientKeysListResponse>("/v1/client-key");
  }

  async createClientKey(data: CreateClientKeyRequest): Promise<void> {
    return this.request<void>("/v1/client-key", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async rotateClientKey(data: CreateClientKeyRequest): Promise<void> {
    return this.request<void>("/v1/client-key", {
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
  async health(): Promise<{ result: string; timestamp: number }> {
    return this.request<{ result: string; timestamp: number }>("/healthz/live");
  }

  // Health check with response time measurement
  async healthWithTiming(): Promise<{
    result: string;
    timestamp: number;
    responseTime: number;
  }> {
    return this.requestWithTiming<{ result: string; timestamp: number }>(
      "/healthz/live",
    );
  }

  // Readiness check
  async readiness(): Promise<{ result: string; timestamp: number }> {
    return this.request<{ result: string; timestamp: number }>(
      "/healthz/ready",
    );
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
  clientKeys: ["clientKeys"] as const,
} as const;
