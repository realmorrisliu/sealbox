import { Badge } from "@/components/ui/badge";

interface StatBadgeProps {
  count: number;
  label: string;
  filtered?: boolean;
}

/**
 * 统计徽章组件
 * 用于显示数据统计和过滤状态
 */
export function StatBadge({ count, label, filtered }: StatBadgeProps) {
  return (
    <div className="flex items-center gap-2">
      <Badge variant="secondary" className="text-xs">
        {count} {label}
      </Badge>
      {filtered && (
        <Badge variant="outline" className="text-xs">
          Filtered
        </Badge>
      )}
    </div>
  );
}
