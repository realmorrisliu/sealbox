import { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface ContentCardProps {
  children: ReactNode;
  className?: string;
  padding?: "none" | "sm" | "md" | "lg";
}

const paddingStyles = {
  none: "",
  sm: "p-4",
  md: "p-6",
  lg: "p-8",
};

/**
 * 统一的内容卡片组件
 * 提供一致的卡片样式和可选的内边距
 */
export function ContentCard({
  children,
  className,
  padding = "md",
}: ContentCardProps) {
  return (
    <div
      className={cn(
        "rounded-xl border bg-background",
        paddingStyles[padding],
        className,
      )}
    >
      {children}
    </div>
  );
}
