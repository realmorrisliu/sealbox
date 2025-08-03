import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

// Import translation resources
import en from '@/locales/en.json';
import zh from '@/locales/zh.json';
import ja from '@/locales/ja.json';
import de from '@/locales/de.json';

export const defaultNS = 'common';
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

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    debug: import.meta.env.DEV,
    fallbackLng: 'en',
    defaultNS,
    ns: ['common'],
    
    resources,
    
    // Language detector options
    detection: {
      order: ['localStorage', 'navigator', 'htmlTag'],
      caches: ['localStorage'],
      lookupLocalStorage: 'sealbox-language',
    },
    
    interpolation: {
      escapeValue: false, // React already handles XSS protection
    },
    
    react: {
      bindI18n: 'languageChanged',
      bindI18nStore: '',
      transEmptyNodeValue: '',
      transSupportBasicHtmlNodes: true,
      transKeepBasicHtmlNodesFor: ['br', 'strong', 'i'],
    },
  });

export default i18n;