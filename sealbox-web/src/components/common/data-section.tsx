import { ReactNode } from "react";
import { ErrorState } from "./error-state";

interface DataSectionProps {
  loading?: boolean;
  error?: any;
  empty?: boolean;
  loadingSkeleton?: ReactNode;
  emptyState?: ReactNode;
  errorProps?: {
    title: string;
    description?: string;
    onRetry?: () => void;
    retryLabel?: string;
  };
  children: ReactNode;
}

/**
 * 数据展示区域组件
 * 统一处理加载、错误、空状态和数据展示的逻辑
 */
export function DataSection({
  loading,
  error,
  empty,
  loadingSkeleton,
  emptyState,
  errorProps,
  children,
}: DataSectionProps) {
  // Loading state
  if (loading && loadingSkeleton) {
    return <>{loadingSkeleton}</>;
  }

  // Error state
  if (error && errorProps) {
    return (
      <ErrorState
        title={errorProps.title}
        description={errorProps.description || error?.message}
        onRetry={errorProps.onRetry}
        retryLabel={errorProps.retryLabel}
      />
    );
  }

  // Empty state
  if (empty && emptyState) {
    return <>{emptyState}</>;
  }

  // Data state
  return <>{children}</>;
}
