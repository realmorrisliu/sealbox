import { Globe } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
  DropdownMenuRadioGroup,
} from "@/components/ui/dropdown-menu";
import { CustomRadioItem } from "@/components/common/custom-radio-item";

const languages = [
  { code: "en", name: "English", icon: "🇺🇸" },
  { code: "zh", name: "中文", icon: "🇨🇳" },
  { code: "ja", name: "日本語", icon: "🇯🇵" },
  { code: "de", name: "Deutsch", icon: "🇩🇪" },
];

/**
 * 语言选择器子菜单
 */
export function LanguageSelector() {
  const { t, i18n } = useTranslation();

  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger className="cursor-pointer">{t("nav.language")}</DropdownMenuSubTrigger>
      <DropdownMenuSubContent className="w-fit">
        <DropdownMenuRadioGroup
          value={i18n.language}
          onValueChange={(v) => {
            i18n.changeLanguage(v);
            if (typeof window !== "undefined") {
              localStorage.setItem("sealbox-language", v);
            }
          }}
        >
          {languages.map((l) => (
            <CustomRadioItem
              key={l.code}
              value={l.code}
              currentValue={i18n.language}
            >
              <span className="mr-2">{l.icon}</span>
              {l.name}
            </CustomRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}
