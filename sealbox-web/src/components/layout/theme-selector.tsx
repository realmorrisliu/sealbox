import { Sun, Moon, Monitor } from "lucide-react";
import {
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
  DropdownMenuRadioGroup,
} from "@/components/ui/dropdown-menu";
import { CustomRadioItem } from "@/components/common/custom-radio-item";
import { useTheme } from "@/components/theme/theme-provider";

/**
 * 主题选择器子菜单
 */
export function ThemeSelector() {
  const { theme, setTheme } = useTheme();

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger className="cursor-pointer">Appearance</DropdownMenuSubTrigger>
      <DropdownMenuSubContent className="w-fit">
        <DropdownMenuRadioGroup value={theme} onValueChange={setTheme as any}>
          <CustomRadioItem value="light" currentValue={theme}>
            <Sun className="h-4 w-4 mr-2" />
            Light
          </CustomRadioItem>
          <CustomRadioItem value="dark" currentValue={theme}>
            <Moon className="h-4 w-4 mr-2" />
            Dark
          </CustomRadioItem>
          <CustomRadioItem value="system" currentValue={theme}>
            <Monitor className="h-4 w-4 mr-2" />
            System
          </CustomRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}
