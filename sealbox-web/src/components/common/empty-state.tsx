import { Button } from "@/components/ui/button";
import { ReactNode } from "react";

interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  actions?: ReactNode;
  children?: ReactNode;
  centered?: boolean;
  withContainer?: boolean;
}

/**
 * 灵活的空状态组件
 * 支持自定义图标、多种操作方式和布局选项
 */
export function EmptyState({
  icon,
  title,
  description,
  action,
  actions,
  children,
  centered = true,
  withContainer = true,
}: EmptyStateProps) {
  const content = (
    <div className={`space-y-5 ${centered ? "text-center" : ""}`}>
      {icon && (
        <div
          className={`w-12 h-12 bg-muted rounded-full flex items-center justify-center text-muted-foreground ${centered ? "mx-auto" : ""}`}
        >
          {icon}
        </div>
      )}
      <div className="space-y-2">
        <h3 className="text-xl font-semibold">{title}</h3>
        {description && (
          <p
            className={`text-base text-muted-foreground ${centered ? "max-w-sm mx-auto" : ""}`}
          >
            {description}
          </p>
        )}
      </div>
      {children}
      {(action || actions) && (
        <div
          className={`flex items-center gap-2 ${centered ? "justify-center" : ""}`}
        >
          {action && (
            <Button onClick={action.onClick} variant="outline">
              {action.label}
            </Button>
          )}
          {actions}
        </div>
      )}
    </div>
  );

  if (withContainer) {
    return (
      <div className="p-14 rounded-xl border bg-background">
        <div
          className={
            centered ? "flex flex-col items-center justify-center" : ""
          }
        >
          {content}
        </div>
      </div>
    );
  }

  return content;
}
