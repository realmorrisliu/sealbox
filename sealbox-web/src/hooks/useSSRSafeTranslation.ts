import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import enTranslations from "@/locales/en.json";

/**
 * SSR-safe translation hook that prevents hydration mismatches
 * by using English translations during initial render
 */
export function useSSRSafeTranslation() {
  const { t, i18n } = useTranslation();
  const [isClient, setIsClient] = useState(false);

  useEffect(() => {
    setIsClient(true);
  }, []);

  // Create a safe translate function
  const safeT = (key: string, fallback?: string): string => {
    if (!isClient) {
      // Use English translation during SSR/first render
      return fallback || getEnglishTranslation(key);
    }
    return t(key);
  };

  return {
    t: safeT,
    i18n,
    isClient,
  };
}

/**
 * Get English translation from the locale file
 * Supports nested keys like "login.errors.authFailed"
 */
function getEnglishTranslation(key: string): string {
  const keys = key.split('.');
  let value: any = enTranslations;
  
  for (const k of keys) {
    if (value && typeof value === 'object' && k in value) {
      value = value[k];
    } else {
      return key; // Return the key itself if translation not found
    }
  }
  
  return typeof value === 'string' ? value : key;
}