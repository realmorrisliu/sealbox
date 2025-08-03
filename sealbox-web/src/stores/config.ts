import { create } from "zustand";
import { persist } from "zustand/middleware";

interface ConfigState {
  // UI 设置
  theme: "light" | "dark" | "system";
  language: "en" | "zh";
  
  // 默认配置
  defaultServerUrl: string;
  
  // Actions
  setTheme: (theme: "light" | "dark" | "system") => void;
  setLanguage: (language: "en" | "zh") => void;
  setDefaultServerUrl: (url: string) => void;
}

export const useConfigStore = create<ConfigState>()(
  persist(
    (set) => ({
      // Initial state
      theme: "system",
      language: "zh",
      defaultServerUrl: "http://localhost:8080",

      // Actions
      setTheme: (theme) => set({ theme }),
      setLanguage: (language) => set({ language }),
      setDefaultServerUrl: (defaultServerUrl) => set({ defaultServerUrl }),
    }),
    {
      name: "sealbox-config", // localStorage key
    },
  ),
);