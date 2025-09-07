import { useEffect } from "react";
import { useTheme } from "@/components/theme/theme-provider";
import { useTranslation } from "react-i18next";

/**
 * Component to synchronize HTML attributes (theme classes and language)
 * This ensures server and client HTML attributes match
 */
export function HtmlAttributes() {
  const { resolvedTheme } = useTheme();
  const { i18n } = useTranslation();

  useEffect(() => {
    // Update theme class
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    root.classList.add(resolvedTheme);
  }, [resolvedTheme]);

  useEffect(() => {
    // Update language attribute based on current i18n language
    const root = document.documentElement;
    root.setAttribute("lang", i18n.language);
  }, [i18n.language]);

  // This component doesn't render anything
  return null;
}
