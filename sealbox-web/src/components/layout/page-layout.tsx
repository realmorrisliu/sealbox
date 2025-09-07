import { ReactNode } from "react";
import { PageHeader } from "./page-header";
import { SearchInput } from "@/components/common/search-input";
import { StatBadge } from "@/components/common/stat-badge";

interface SearchProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  size?: "sm" | "md" | "lg";
}

interface StatsProps {
  count: number;
  label: string;
  filtered?: boolean;
}

interface PageLayoutProps {
  title: string;
  subtitle?: string;
  stats?: StatsProps;
  searchProps?: SearchProps;
  actions?: ReactNode;
  meta?: ReactNode;
  children: ReactNode;
}

/**
 * 页面布局组件
 * 整合 PageHeader 和常用的页面元素（搜索、统计、操作）
 */
export function PageLayout({
  title,
  subtitle,
  stats,
  searchProps,
  actions,
  meta,
  children,
}: PageLayoutProps) {
  // 构建 meta 信息
  const metaContent =
    meta ||
    (stats && (
      <StatBadge
        count={stats.count}
        label={stats.label}
        filtered={stats.filtered}
      />
    ));

  // 构建 actions 区域
  const actionsContent = (
    <div className="flex items-center gap-3">
      {searchProps && (
        <SearchInput
          value={searchProps.value}
          onChange={searchProps.onChange}
          placeholder={searchProps.placeholder || "Search..."}
          size={searchProps.size}
        />
      )}
      {actions}
    </div>
  );

  return (
    <div className="space-y-6">
      <PageHeader
        title={title}
        subtitle={subtitle}
        meta={metaContent}
        actions={actionsContent}
      />
      {children}
    </div>
  );
}
