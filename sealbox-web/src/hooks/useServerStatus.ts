import { useHealthCheck } from "./use-api";
import { useAuthStore } from "@/stores/auth";

export type ServerStatus = "connected" | "disconnected" | "connecting";

export interface ServerStatusData {
  url: string;
  status: ServerStatus;
  responseTime?: number;
  refresh: () => void;
}

export function useServerStatus() {
  const { serverUrl } = useAuthStore();
  const { data, error, isLoading, refetch } = useHealthCheck();

  const getStatus = (): ServerStatus => {
    if (isLoading) return "connecting";
    if (error) return "disconnected";
    if (data) return "connected";
    return "disconnected";
  };

  const cleanUrl = (url?: string) => {
    if (!url) return "Not connected";
    return url.replace(/^https?:\/\//, "").replace(/\/$/, "");
  };

  return {
    url: cleanUrl(serverUrl),
    status: getStatus(),
    responseTime: data?.responseTime,
    refresh: refetch,
  } as ServerStatusData;
}
