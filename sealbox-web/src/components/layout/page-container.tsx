import { ReactNode } from "react";
import { AuthGuard } from "@/components/auth/auth-guard";
import { MainLayout } from "@/components/layout/main-layout";

interface PageContainerProps {
  children: ReactNode;
}

/**
 * 页面容器组件，整合 AuthGuard 和 MainLayout
 * 为所有需要认证的页面提供统一的包装结构
 */
export function PageContainer({ children }: PageContainerProps) {
  return (
    <AuthGuard>
      <MainLayout>{children}</MainLayout>
    </AuthGuard>
  );
}
