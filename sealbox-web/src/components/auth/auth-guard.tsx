import { useEffect } from "react";
import { useRouter } from "@tanstack/react-router";
import { useAuthStore } from "@/stores/auth";
import { ThemeToggle } from "@/components/theme/theme-toggle";
import { LanguageSelector } from "@/components/i18n/language-selector";
import { useSSRSafeTranslation } from "@/hooks/useSSRSafeTranslation";

interface AuthGuardProps {
  children: React.ReactNode;
  redirectTo?: string;
}

export function AuthGuard({ children, redirectTo = "/login" }: AuthGuardProps) {
  const { isAuthenticated } = useAuthStore();
  const router = useRouter();
  const { t } = useSSRSafeTranslation();

  useEffect(() => {
    if (!isAuthenticated) {
      router.navigate({ to: redirectTo });
    }
  }, [isAuthenticated, router, redirectTo]);

  // If not authenticated, show loading state with branding
  if (!isAuthenticated) {
    return (
      <div className="min-h-screen bg-background flex flex-col">
        {/* Top bar with controls */}
        <div className="flex justify-end p-4">
          <div className="flex items-center space-x-1">
            <ThemeToggle />
            <LanguageSelector />
          </div>
        </div>
        
        {/* Main content */}
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center space-y-4">
            {/* Brand Logo */}
            <div className="flex items-center justify-center space-x-3 mb-6">
              <div className="text-2xl">🦭</div>
              <h1 className="text-2xl font-bold text-foreground">sealbox</h1>
            </div>
            
            {/* Loading Animation */}
            <div className="flex items-center justify-center space-x-2">
              <div className="w-6 h-6 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
              <p className="text-muted-foreground">{t("common.loading")}</p>
            </div>
            
            {/* Optional hint */}
            <p className="text-sm text-muted-foreground/60">
              {t("auth.redirecting")}
            </p>
          </div>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
