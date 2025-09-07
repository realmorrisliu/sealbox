import { ReactNode } from "react";
import { Check } from "lucide-react";
import { DropdownMenuRadioItem } from "@/components/ui/dropdown-menu";

interface CustomRadioItemProps {
  value: string;
  currentValue: string;
  children: ReactNode;
  onSelect?: () => void;
}

/**
 * 自定义单选菜单项组件
 * 显示右侧对勾，点击不关闭菜单
 */
export function CustomRadioItem({
  value,
  currentValue,
  children,
  onSelect,
}: CustomRadioItemProps) {
  return (
    <DropdownMenuRadioItem
      value={value}
      className="pr-2 pl-2 [&>span:first-child]:hidden justify-between cursor-pointer"
      onSelect={(e) => {
        e.preventDefault();
        onSelect?.();
      }}
    >
      <div className="flex items-center">{children}</div>
      <div className="ml-auto">
        {currentValue === value && <Check className="h-4 w-4" />}
      </div>
    </DropdownMenuRadioItem>
  );
}
