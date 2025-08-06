import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

// Import translation resources
import en from "@/locales/en.json";
import zh from "@/locales/zh.json";
import ja from "@/locales/ja.json";
import de from "@/locales/de.json";

export const defaultNS = "common";
export const resources = {
  en: {
    common: en,
  },
  zh: {
    common: zh,
  },
  ja: {
    common: ja,
  },
  de: {
    common: de,
  },
} as const;

// Get initial language from localStorage or browser preference
const getInitialLanguage = (): string => {
  if (typeof window !== "undefined") {
    // First check localStorage
    const stored = localStorage.getItem("sealbox-language");
    if (stored && ["en", "zh", "ja", "de"].includes(stored)) {
      return stored;
    }
    
    // Then check browser language
    const browserLang = navigator.language?.split('-')[0];
    if (browserLang && ["en", "zh", "ja", "de"].includes(browserLang)) {
      return browserLang;
    }
  }
  
  return "en"; // SSR fallback
};

// Initialize i18n with immediate sync initialization
i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    debug: import.meta.env.DEV,
    lng: getInitialLanguage(),
    fallbackLng: "en",
    defaultNS,
    ns: ["common"],

    resources,

    // Language detector options  
    detection: {
      order: ["localStorage", "navigator", "htmlTag"],
      caches: ["localStorage"],
      lookupLocalStorage: "sealbox-language",
    },

    // Critical: disable React Suspense to avoid hydration issues
    react: {
      useSuspense: false,
      bindI18n: "languageChanged",
      bindI18nStore: "",
      transEmptyNodeValue: "",
      transSupportBasicHtmlNodes: true,
      transKeepBasicHtmlNodesFor: ["br", "strong", "i"],
    },

    interpolation: {
      escapeValue: false, // React already handles XSS protection
    },
  });

export default i18n;
