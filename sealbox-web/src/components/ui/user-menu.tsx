import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useRouter } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { LanguageSelector } from "@/components/ui/language-selector";
import { ThemeToggle } from "@/components/ui/theme-toggle";
import { useAuthStore } from "@/stores/auth";
import { useHealthCheck } from "@/hooks/use-api";
import { 
  ChevronDown, 
  Settings, 
  LogOut, 
  Activity,
  Server,
  Globe,
  Palette
} from "lucide-react";

export function UserMenu() {
  const [isOpen, setIsOpen] = useState(false);
  const router = useRouter();
  const { logout, serverUrl } = useAuthStore();
  const { data: healthStatus, isLoading: isHealthChecking } = useHealthCheck();
  const { t } = useTranslation();

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

  const handleLogout = () => {
    logout();
    setIsOpen(false);
    router.navigate({ to: "/login" });
  };

  return (
    <div className="relative">
      {/* Trigger Button */}
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center space-x-2 h-8 px-3 hover:bg-accent transition-colors duration-150"
      >
        <Badge 
          variant={serverStatus.variant}
          className="flex items-center space-x-1 px-1.5 py-0.5 text-xs"
        >
          {serverStatus.status === 'checking' ? (
            <div className="w-2 h-2 border border-foreground/40 border-t-foreground rounded-full animate-spin" />
          ) : (
            <Activity className="h-2 w-2" />
          )}
        </Badge>
        <ChevronDown className={`h-3 w-3 transition-transform duration-150 ${isOpen ? 'rotate-180' : ''}`} />
      </Button>

      {/* Dropdown Menu */}
      {isOpen && (
        <>
          {/* Backdrop */}
          <div 
            className="fixed inset-0 z-40" 
            onClick={() => setIsOpen(false)}
          />
          
          {/* Menu Content - Responsive positioning */}
          <div className="absolute right-0 sm:right-0 top-full mt-2 w-[calc(100vw-2rem)] max-w-80 sm:w-72 bg-card border border-border rounded-lg shadow-lg z-50">
            {/* Server Status Section */}
            <div className="p-4 space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Server Status</span>
                <Badge variant={serverStatus.variant} className="text-xs">
                  {t(`status.${serverStatus.status}`)}
                </Badge>
              </div>
              
              {serverUrl && (
                <div className="flex items-center space-x-2 text-xs text-muted-foreground">
                  <Server className="h-3 w-3" />
                  <span className="font-mono">{new URL(serverUrl).host}</span>
                </div>
              )}
            </div>

            <Separator />

            {/* Settings Section */}
            <div className="p-4 space-y-3">
              <div className="flex items-center space-x-2 text-sm font-medium">
                <Settings className="h-4 w-4" />
                <span>Settings</span>
              </div>
              
              {/* Language Selector */}
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-2 text-sm">
                  <Globe className="h-3 w-3" />
                  <span>Language</span>
                </div>
                <LanguageSelector />
              </div>

              {/* Theme Toggle */}
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-2 text-sm">
                  <Palette className="h-3 w-3" />
                  <span>Theme</span>
                </div>
                <ThemeToggle />
              </div>
            </div>

            <Separator />

            {/* Actions Section */}
            <div className="p-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={handleLogout}
                className="w-full justify-start text-sm hover:bg-destructive/10 hover:text-destructive transition-colors duration-150"
              >
                <LogOut className="h-4 w-4 mr-2" />
                {t('nav.logout')}
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}