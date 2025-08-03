import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error: any) => {
        // 不重试认证错误
        if (error?.status === 401 || error?.status === 403) {
          return false;
        }
        // 其他错误重试最多2次
        return failureCount < 2;
      },
      staleTime: 5 * 60 * 1000, // 5分钟
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: false,
    },
  },
});