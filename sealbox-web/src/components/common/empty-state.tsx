import { Button } from "@/components/ui/button";
import { LucideIcon } from "lucide-react";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
}: EmptyStateProps) {
  return (
    <div className="p-14 rounded-xl border bg-background">
      <div className="flex flex-col items-center justify-center text-center space-y-5">
        <div className="w-24 h-24 bg-muted rounded-full flex items-center justify-center">
          <Icon className="h-10 w-10 text-muted-foreground" />
        </div>
        <div className="space-y-2">
          <h3 className="text-xl font-semibold">{title}</h3>
          {description && (
            <p className="text-base text-muted-foreground max-w-sm mx-auto">
              {description}
            </p>
          )}
        </div>
        {action && (
          <Button onClick={action.onClick} variant="outline">
            {action.label}
          </Button>
        )}
      </div>
    </div>
  );
}
