import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Globe, Check } from "lucide-react";

export function LanguageSelector() {
  const { i18n, t } = useTranslation();

  const languages = [
    { code: "en", name: t("language.english"), flag: "🇺🇸" },
    { code: "zh", name: t("language.chinese"), flag: "🇨🇳" },
    { code: "ja", name: t("language.japanese"), flag: "🇯🇵" },
    { code: "de", name: t("language.german"), flag: "🇩🇪" },
  ];

  const currentLanguage =
    languages.find((lang) => lang.code === i18n.language) || languages[0];

  const handleLanguageChange = (languageCode: string) => {
    i18n.changeLanguage(languageCode);
    // Manually save to localStorage as backup
    if (typeof window !== "undefined") {
      localStorage.setItem("sealbox-language", languageCode);
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
          <Globe className="w-4 h-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-40">
        {languages.map((language) => (
          <DropdownMenuItem
            key={language.code}
            onClick={() => handleLanguageChange(language.code)}
            className="flex items-center justify-between text-xs cursor-pointer"
          >
            <div className="flex items-center space-x-2">
              <span>{language.flag}</span>
              <span>{language.name}</span>
            </div>
            {language.code === i18n.language && <Check className="w-3 h-3" />}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
