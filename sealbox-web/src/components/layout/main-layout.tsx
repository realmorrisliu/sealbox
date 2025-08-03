import { Link, useRouter } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { LanguageSelector } from "@/components/ui/language-selector";
import { ThemeToggle } from "@/components/ui/theme-toggle";
import { LayeredBackground } from "@/components/ui/layered-background";
import { SealboxLogo } from "@/components/ui/sealbox-logo";
import { useAuthStore } from "@/stores/auth";
import { useHealthCheck } from "@/hooks/use-api";
import { Settings, LogOut, Activity } from "lucide-react";

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { logout, serverUrl } = useAuthStore();
  const router = useRouter();
  const { data: healthStatus, isLoading: isHealthChecking, error: healthError } = useHealthCheck();
  const { t } = useTranslation();

  const handleLogout = () => {
    logout();
    router.navigate({ to: "/login" });
  };

  // Determine server status
  const getServerStatus = () => {
    if (isHealthChecking) {
      return { status: 'checking', text: 'Checking...', variant: 'secondary' as const };
    }
    if (healthStatus) {
      return { status: 'online', text: 'Online', variant: 'default' as const };
    }
    return { status: 'offline', text: 'Offline', variant: 'destructive' as const };
  };

  const serverStatus = getServerStatus();

  return (
    <LayeredBackground 
      variant="subtle" 
      texture="grid" 
      animated={false}
    >
      {/* Top navigation bar */}
      <header className="sticky top-0 z-50 bg-glass-bright border-b border-border/30">
        <div className="container mx-auto px-4 sm:px-6 py-3 sm:py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4 sm:space-x-8">
              <Link to="/" className="group">
                <SealboxLogo size="md" className="group-hover:scale-105 transition-transform" />
              </Link>
              
              <nav className="hidden sm:flex items-center space-x-1">
                <Link
                  to="/"
                  className="px-3 py-2 text-sm font-medium rounded-md hover:bg-gradient-warm transition-all"
                  activeProps={{ className: "bg-gradient-vibrant text-primary border border-primary/20" }}
                >
                  {t('nav.secretManagement')}
                </Link>
                {/* Temporarily hide unimplemented pages */}
                {/* 
                <Link
                  to="/keys"
                  className="px-3 py-2 text-sm font-medium rounded-md hover:bg-accent/50 transition-colors"
                  activeProps={{ className: "bg-accent text-primary" }}
                >
                  Master Keys
                </Link>
                <Link
                  to="/admin"
                  className="px-3 py-2 text-sm font-medium rounded-md hover:bg-accent/50 transition-colors"
                  activeProps={{ className: "bg-accent text-primary" }}
                >
                  Administration
                </Link>
                */}
              </nav>
            </div>

            <div className="flex items-center space-x-2 sm:space-x-3">
              {/* Server status */}
              <div className="flex items-center space-x-2">
                <Badge 
                  variant={serverStatus.variant}
                  className={`flex items-center space-x-1 px-2 py-1 ${
                    serverStatus.status === 'online' ? 'bg-status-success' : 
                    serverStatus.status === 'offline' ? 'bg-status-error' : 
                    'bg-status-checking'
                  } border-0`}
                >
                  {serverStatus.status === 'checking' ? (
                    <div className="w-3 h-3 border border-foreground/40 border-t-foreground rounded-full animate-spin" />
                  ) : (
                    <Activity className="h-3 w-3" />
                  )}
                  <span className="text-xs hidden sm:inline">
                    {t(`status.${serverStatus.status}`)}
                  </span>
                </Badge>
                <span className="text-xs text-muted-foreground hidden lg:inline">
                  {serverUrl ? new URL(serverUrl).host : "Not connected"}
                </span>
              </div>

              <Separator orientation="vertical" className="h-6 hidden sm:block" />

              {/* Theme toggle */}
              <ThemeToggle />

              {/* Language selector */}
              <LanguageSelector />

              {/* Settings button - hidden on mobile */}
              <Button variant="ghost" size="icon" className="hover:bg-accent/50 transition-colors hidden sm:flex">
                <Settings className="h-4 w-4" />
              </Button>

              {/* Logout button */}
              <Button 
                variant="ghost" 
                size="icon" 
                onClick={handleLogout} 
                title={t('nav.logout')}
                className="hover:bg-destructive/10 hover:text-destructive transition-colors"
              >
                <LogOut className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      </header>

      {/* Main content area */}
      <main className="container mx-auto px-4 sm:px-6 py-6 sm:py-8 space-section">
        {children}
      </main>
    </LayeredBackground>
  );
}