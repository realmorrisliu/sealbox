import { useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { createApiClient, queryKeys } from "@/lib/api";
import { useAuthStore } from "@/stores/auth";
import type { CreateSecretRequest, CreateClientKeyRequest } from "@/lib/types";

// Hook for creating API client
export function useApiClient() {
  const { token, serverUrl } = useAuthStore();

  return useMemo(() => {
    if (!serverUrl) return null;
    return createApiClient(serverUrl, token);
  }, [serverUrl, token]);
}

// Secret-related hooks
export function useSecrets() {
  const apiClient = useApiClient();

  return useQuery({
    queryKey: queryKeys.secrets,
    queryFn: () => apiClient?.listSecrets(),
    enabled: !!apiClient,
  });
}

export function useSecret(
  key: string,
  version?: number,
  options?: { enabled?: boolean },
) {
  const apiClient = useApiClient();

  return useQuery({
    queryKey: queryKeys.secret(key, version),
    queryFn: () => apiClient?.getSecret(key, version),
    enabled: !!apiClient && !!key && options?.enabled !== false,
  });
}

export function useCreateSecret() {
  const apiClient = useApiClient();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ key, data }: { key: string; data: CreateSecretRequest }) =>
      apiClient!.createSecret(key, data),
    onSuccess: () => {
      // Refresh secrets list
      queryClient.invalidateQueries({ queryKey: queryKeys.secrets });
    },
  });
}

export function useDeleteSecret() {
  const apiClient = useApiClient();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ key, version }: { key: string; version: number }) =>
      apiClient!.deleteSecret(key, version),
    onSuccess: () => {
      // Refresh secrets list
      queryClient.invalidateQueries({ queryKey: queryKeys.secrets });
    },
  });
}


// Client key related hooks
export function useClientKeys() {
  const apiClient = useApiClient();

  return useQuery({
    queryKey: queryKeys.clientKeys,
    queryFn: () => apiClient?.listClientKeys(),
    enabled: !!apiClient,
  });
}

export function useCreateClientKey() {
  const apiClient = useApiClient();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateClientKeyRequest) =>
      apiClient!.createClientKey(data),
    onSuccess: () => {
      // Refresh client keys list
      queryClient.invalidateQueries({ queryKey: queryKeys.clientKeys });
    },
  });
}

export function useRotateClientKey() {
  const apiClient = useApiClient();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateClientKeyRequest) =>
      apiClient!.rotateClientKey(data),
    onSuccess: () => {
      // Refresh client keys and all secrets (affects encryption)
      queryClient.invalidateQueries({ queryKey: queryKeys.clientKeys });
      queryClient.invalidateQueries({ queryKey: queryKeys.secrets });
    },
  });
}

// Admin functionality hooks
export function useCleanupExpiredSecrets() {
  const apiClient = useApiClient();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => apiClient!.cleanupExpiredSecrets(),
    onSuccess: () => {
      // Refresh secrets list
      queryClient.invalidateQueries({ queryKey: queryKeys.secrets });
    },
  });
}

// Health check hook with manual trigger
export function useHealthCheck() {
  const apiClient = useApiClient();

  return useQuery({
    queryKey: ["health"],
    queryFn: () => apiClient?.healthWithTiming(),
    enabled: false, // Disabled by default, trigger manually
    retry: 1,
    staleTime: 30000, // Consider data stale after 30 seconds
    gcTime: 60000, // Keep in cache for 1 minute
  });
}
