import { useEffect, useState } from "react";
import { useHealthCheck } from "./use-api";
import { useAuthStore } from "@/stores/auth";

export type ServerStatus = "connected" | "disconnected" | "connecting";

export interface ServerStatusData {
  url: string;
  status: ServerStatus;
  responseTime?: number;
}

export function useServerStatus() {
  const { serverUrl } = useAuthStore();
  const { data, error, isLoading } = useHealthCheck();
  const [responseTime, setResponseTime] = useState<number>();
  const [lastCheckTime, setLastCheckTime] = useState<number>();

  useEffect(() => {
    if (data && lastCheckTime) {
      const timeDiff = Date.now() - lastCheckTime;
      setResponseTime(Math.min(timeDiff, 9999));
    }
  }, [data, lastCheckTime]);

  useEffect(() => {
    setLastCheckTime(Date.now());
  }, [isLoading]);

  const getStatus = (): ServerStatus => {
    if (isLoading && !data) return "connecting";
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
    responseTime,
  } as ServerStatusData;
}