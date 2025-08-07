"use client";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Badge } from "@/components/ui/badge";
import { LanguageSelector } from "@/components/i18n/language-selector";
import { ThemeToggle } from "@/components/theme/theme-toggle";
import {
  LogOut,
  ChevronDown,
  Server,
  WifiOff,
  Loader2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAuthStore } from "@/stores/auth";
import { useServerStatus } from "@/hooks/useServerStatus";

export function Navigation() {
  const { t } = useTranslation();
  const { logout, serverUrl } = useAuthStore();
  const serverStatus = useServerStatus();

  const getStatusColor = (status: string) => {
    switch (status) {
      case "connected":
        return "text-green-500";
      case "connecting":
        return "text-yellow-500";
      case "disconnected":
        return "text-red-500";
      default:
        return "text-gray-500";
    }
  };

  return (
    <nav className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="w-full px-4">
        <div className="flex h-12 items-center justify-between">
          {/* Logo and Brand */}
          <div className="flex items-center space-x-2">
            <div className="flex items-center space-x-2">
              <div className="text-lg">🦭</div>
              <div className="flex items-center space-x-1">
                <span className="text-lg font-bold text-foreground">
                  sealbox
                </span>
                <Badge
                  variant="secondary"
                  className="text-xs px-1.5 py-0.5 h-5"
                >
                  BETA
                </Badge>
              </div>
            </div>
          </div>

          {/* Right Side Controls */}
          <div className="flex items-center space-x-0.5">
            {/* Language Selector */}
            <LanguageSelector />

            {/* Theme Selector */}
            <ThemeToggle />

            {/* User Menu */}
            <DropdownMenu
              onOpenChange={(isOpen) => {
                if (isOpen) {
                  serverStatus.refresh();
                }
              }}
            >
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="h-8 px-2">
                  <div className="w-5 h-5 rounded bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-xs font-medium mr-1">
                    {serverUrl ? serverUrl.charAt(0).toUpperCase() : "S"}
                  </div>
                  <ChevronDown className="w-3 h-3" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-48">
                <div className="relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none">
                  <Server className="pointer-events-none shrink-0 size-4 text-muted-foreground" />
                  <div className="flex-1 min-w-0 flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <p className="text-xs font-medium truncate">
                        {serverStatus.url}
                      </p>
                    </div>
                    <div className={`${getStatusColor(serverStatus.status)} shrink-0 text-xs font-medium flex items-center`}>
                      {serverStatus.status === "connected" && serverStatus.responseTime ? (
                        `${serverStatus.responseTime}ms`
                      ) : serverStatus.status === "connecting" ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <WifiOff className="h-3 w-3" />
                      )}
                    </div>
                  </div>
                </div>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  className="text-xs text-red-600 cursor-pointer"
                  onClick={logout}
                >
                  <LogOut className="pointer-events-none shrink-0 size-4" />
                  <span>{t("nav.signOut")}</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>
    </nav>
  );
}
