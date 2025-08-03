import { useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { createApiClient, queryKeys } from "@/lib/api";
import { useAuthStore } from "@/stores/auth";
import type { CreateSecretRequest, CreateMasterKeyRequest } from "@/lib/types";

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

export function useSecret(key: string, version?: number) {
  const apiClient = useApiClient();
  
  return useQuery({
    queryKey: queryKeys.secret(key, version),
    queryFn: () => apiClient?.getSecret(key, version),
    enabled: !!apiClient && !!key,
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

// Master key related hooks
export function useMasterKeys() {
  const apiClient = useApiClient();
  
  return useQuery({
    queryKey: queryKeys.masterKeys,
    queryFn: () => apiClient?.listMasterKeys(),
    enabled: !!apiClient,
  });
}

export function useCreateMasterKey() {
  const apiClient = useApiClient();
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: (data: CreateMasterKeyRequest) =>
      apiClient!.createMasterKey(data),
    onSuccess: () => {
      // Refresh master keys list
      queryClient.invalidateQueries({ queryKey: queryKeys.masterKeys });
    },
  });
}

export function useRotateMasterKey() {
  const apiClient = useApiClient();
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: (data: CreateMasterKeyRequest) =>
      apiClient!.rotateMasterKey(data),
    onSuccess: () => {
      // Refresh master keys and all secrets (affects encryption)
      queryClient.invalidateQueries({ queryKey: queryKeys.masterKeys });
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

// Health check hook
export function useHealthCheck() {
  const apiClient = useApiClient();
  
  return useQuery({
    queryKey: ["health"],
    queryFn: () => apiClient?.health(),
    enabled: !!apiClient,
    retry: 1,
    refetchInterval: 30000, // Check every 30 seconds
  });
}