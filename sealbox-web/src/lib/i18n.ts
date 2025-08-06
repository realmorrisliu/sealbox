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

// Supported languages
export const supportedLanguages = ["en", "zh", "ja", "de"] as const;
export type SupportedLanguage = typeof supportedLanguages[number];

// Get initial language for SSR-safe initialization
export const getInitialLanguage = (): SupportedLanguage => {
  // Always default to English for SSR consistency
  // LanguageDetector will handle client-side detection after hydration
  return "en";
};

// Initialize i18n optimized for SSR
const isSSR = typeof window === "undefined";

// Initialize with LanguageDetector - let it handle language detection
i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    debug: import.meta.env.DEV,
    // Only set lng explicitly on SSR, let LanguageDetector handle client-side
    ...(isSSR ? { lng: getInitialLanguage() } : {}),
    fallbackLng: "en",
    defaultNS,
    ns: ["common"],

    resources,

    // Language detector options (SSR-safe)
    detection: {
      // Only use localStorage and navigator for client-side detection
      order: isSSR ? [] : ["localStorage", "navigator"],
      // CRITICAL: Disable caches to prevent overwriting localStorage
      caches: [],
      lookupLocalStorage: "sealbox-language",
    },

    // SSR-optimized React settings
    react: {
      useSuspense: false, // Critical: disable Suspense for SSR
      bindI18n: "languageChanged loaded",
      bindI18nStore: "added removed",
      transEmptyNodeValue: "",
      transSupportBasicHtmlNodes: true,
      transKeepBasicHtmlNodesFor: ["br", "strong", "i", "b", "em"],
    },

    interpolation: {
      escapeValue: false, // React already handles XSS protection
    },

    // SSR specific options
    initImmediate: !isSSR, // Initialize immediately on client side
    cleanCode: true, // Clean up language codes
    
    // Preload languages for better SSR performance
    preload: supportedLanguages,
  });

// Force language detection on client side after hydration
if (typeof window !== "undefined") {
  // Wait for DOM to be ready, then manually trigger detection
  const checkAndSetLanguage = () => {
    const stored = localStorage.getItem("sealbox-language");
    console.log("Debug: localStorage language:", stored);
    console.log("Debug: i18n.language before:", i18n.language);
    
    if (stored && supportedLanguages.includes(stored as SupportedLanguage)) {
      if (stored !== i18n.language) {
        console.log("Debug: Changing language from", i18n.language, "to", stored);
        i18n.changeLanguage(stored);
      }
    }
  };

  // Try multiple approaches to ensure language detection works
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", checkAndSetLanguage);
  } else {
    // DOM already loaded
    setTimeout(checkAndSetLanguage, 0);
  }
  
  // Also listen for i18n events to debug
  i18n.on('initialized', () => {
    console.log("Debug: i18n initialized with language:", i18n.language);
    checkAndSetLanguage();
  });
  
  i18n.on('languageChanged', (lng) => {
    console.log("Debug: Language changed to:", lng);
  });
}

export default i18n;
