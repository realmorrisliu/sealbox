import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Sun, Moon, Monitor, Check } from "lucide-react";
import { useTheme } from "./theme-provider";
import { useSSRSafeTranslation } from "@/hooks/useSSRSafeTranslation";

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const { t } = useSSRSafeTranslation();

  const themes = [
    { value: "light", label: t("theme.light"), icon: Sun },
    { value: "dark", label: t("theme.dark"), icon: Moon },
    { value: "system", label: t("theme.system"), icon: Monitor },
  ] as const;

  const currentTheme = themes.find((t) => t.value === theme) || themes[2];
  const ThemeIcon = currentTheme.icon;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
          <ThemeIcon className="h-4 w-4" />
          <span className="sr-only">{t("theme.toggle")}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-32">
        {themes.map((themeOption) => {
          const IconComponent = themeOption.icon;
          return (
            <DropdownMenuItem
              key={themeOption.value}
              onClick={() => setTheme(themeOption.value)}
              className="flex items-center justify-between text-xs cursor-pointer"
            >
              <div className="flex items-center space-x-2">
                <IconComponent className="h-3 w-3" />
                <span>{themeOption.label}</span>
              </div>
              {themeOption.value === theme && <Check className="w-3 h-3" />}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
