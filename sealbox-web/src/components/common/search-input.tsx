import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

interface SearchInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const sizeStyles = {
  sm: "h-8 w-48",
  md: "h-9 w-56",
  lg: "h-10 w-64",
};

/**
 * 统一的搜索输入框组件
 * 提供一致的搜索框样式和行为
 */
export function SearchInput({
  value,
  onChange,
  placeholder,
  size = "lg",
  className,
}: SearchInputProps) {
  return (
    <Input
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className={cn(sizeStyles[size], className)}
    />
  );
}
