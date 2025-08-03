import { Link, useRouter } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { LanguageSelector } from "@/components/ui/language-selector";
import { useAuthStore } from "@/stores/auth";
import { useHealthCheck } from "@/hooks/use-api";
import { KeyRound, Server, Settings, LogOut, Activity } from "lucide-react";

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { logout, serverUrl } = useAuthStore();
  const router = useRouter();
  const { data: healthStatus } = useHealthCheck();
  const { t } = useTranslation();

  const handleLogout = () => {
    logout();
    router.navigate({ to: "/login" });
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Top navigation bar */}
      <header className="border-b bg-card">
        <div className="container mx-auto px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-6">
              <Link to="/" className="flex items-center space-x-2">
                <KeyRound className="h-6 w-6" />
                <span className="text-xl font-bold">{t('app.title')}</span>
              </Link>
              
              <nav className="flex items-center space-x-4">
                <Link
                  to="/"
                  className="text-sm font-medium hover:text-primary"
                  activeProps={{ className: "text-primary" }}
                >
                  {t('nav.secretManagement')}
                </Link>
                {/* Temporarily hide unimplemented pages */}
                {/* 
                <Link
                  to="/keys"
                  className="text-sm font-medium hover:text-primary"
                  activeProps={{ className: "text-primary" }}
                >
                  Master Keys
                </Link>
                <Link
                  to="/admin"
                  className="text-sm font-medium hover:text-primary"
                  activeProps={{ className: "text-primary" }}
                >
                  Administration
                </Link>
                */}
              </nav>
            </div>

            <div className="flex items-center space-x-4">
              {/* Server status */}
              <div className="flex items-center space-x-2 text-sm text-muted-foreground">
                <Activity
                  className={`h-4 w-4 ${
                    healthStatus ? "text-green-500" : "text-red-500"
                  }`}
                />
                <span className="hidden sm:inline">
                  {serverUrl ? new URL(serverUrl).host : "Not connected"}
                </span>
              </div>

              {/* Language selector */}
              <LanguageSelector />

              {/* Settings button */}
              <Button variant="ghost" size="icon">
                <Settings className="h-4 w-4" />
              </Button>

              {/* Logout button */}
              <Button variant="ghost" size="icon" onClick={handleLogout} title={t('nav.logout')}>
                <LogOut className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      </header>

      {/* Main content area */}
      <main className="container mx-auto px-4 py-6">
        {children}
      </main>
    </div>
  );
}