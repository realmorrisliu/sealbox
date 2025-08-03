import { useEffect } from "react";
import { useRouter } from "@tanstack/react-router";
import { useAuthStore } from "@/stores/auth";

interface AuthGuardProps {
  children: React.ReactNode;
  redirectTo?: string;
}

export function AuthGuard({ children, redirectTo = "/login" }: AuthGuardProps) {
  const { isAuthenticated } = useAuthStore();
  const router = useRouter();

  useEffect(() => {
    if (!isAuthenticated) {
      router.navigate({ to: redirectTo });
    }
  }, [isAuthenticated, router, redirectTo]);

  // 如果未认证，显示空白或加载状态
  if (!isAuthenticated) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <p className="text-muted-foreground">重定向到登录页...</p>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}