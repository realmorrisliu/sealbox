import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { AuthState } from "@/lib/types";

interface AuthStore extends AuthState {
  // Actions
  login: (token: string, serverUrl: string) => void;
  logout: () => void;
  setServerUrl: (url: string) => void;
  setToken: (token: string) => void;
}

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      // Initial state
      isAuthenticated: false,
      token: undefined,
      serverUrl: undefined,

      // Actions
      login: (token: string, serverUrl: string) => {
        set({
          isAuthenticated: true,
          token,
          serverUrl,
        });
      },

      logout: () => {
        set({
          isAuthenticated: false,
          token: undefined,
          serverUrl: undefined,
        });
      },

      setServerUrl: (serverUrl: string) => {
        set({ serverUrl });
      },

      setToken: (token: string) => {
        set({
          token,
          isAuthenticated: !!token,
        });
      },
    }),
    {
      name: "sealbox-auth", // localStorage key
      partialize: (state) => ({
        token: state.token,
        serverUrl: state.serverUrl,
        isAuthenticated: state.isAuthenticated,
      }),
    },
  ),
);
