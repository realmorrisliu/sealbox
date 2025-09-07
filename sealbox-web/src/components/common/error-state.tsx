import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { AlertTriangle } from "lucide-react";

interface ErrorStateProps {
  title: string;
  description?: string;
  onRetry?: () => void;
  retryLabel?: string;
}

/**
 * 错误状态组件
 * 用于统一显示错误信息和重试操作
 */
export function ErrorState({
  title,
  description,
  onRetry,
  retryLabel = "Retry",
}: ErrorStateProps) {
  return (
    <div className="space-y-4">
      <Alert variant="destructive">
        <AlertTriangle className="h-4 w-4" />
        <div>
          <p className="font-medium">{title}</p>
          {description && <p className="text-sm mt-1">{description}</p>}
        </div>
      </Alert>
      {onRetry && <Button onClick={onRetry}>{retryLabel}</Button>}
    </div>
  );
}
