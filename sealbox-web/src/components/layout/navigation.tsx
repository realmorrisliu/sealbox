"use client";

import { useState } from "react";
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
  Settings,
  LogOut,
  ChevronDown,
  Circle,
  Server,
  Wifi,
  WifiOff,
} from "lucide-react";
import { useTranslation } from "react-i18next";

interface ServerStatus {
  url: string;
  status: "connected" | "disconnected" | "connecting";
  responseTime?: number;
  version?: string;
}

export function Navigation() {
  const { t } = useTranslation();
  const [serverStatus] = useState<ServerStatus>({
    url: "localhost:8080",
    status: "connected",
    responseTime: 45,
    version: "v1.2.3",
  });

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

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "connected":
        return <Wifi className="w-3 h-3" />;
      case "connecting":
        return <Server className="w-3 h-3" />;
      case "disconnected":
        return <WifiOff className="w-3 h-3" />;
      default:
        return <Circle className="w-3 h-3" />;
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

          {/* Server Status - Compact */}
          <div className="hidden md:flex items-center space-x-1.5 px-1.5 py-0.5 rounded-md bg-muted/50">
            <div className={`${getStatusColor(serverStatus.status)}`}>
              {getStatusIcon(serverStatus.status)}
            </div>
            <span className="text-xs font-mono">{serverStatus.url}</span>
            {serverStatus.responseTime && (
              <span className="text-xs text-muted-foreground">
                {serverStatus.responseTime}ms
              </span>
            )}
          </div>

          {/* Right Side Controls */}
          <div className="flex items-center space-x-0.5">
            {/* Language Selector */}
            <LanguageSelector />

            {/* Theme Selector */}
            <ThemeToggle />

            {/* Settings */}
            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
              <Settings className="w-4 h-4" />
            </Button>

            {/* User Menu */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="h-8 px-2">
                  <div className="w-5 h-5 rounded bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-xs font-medium mr-1">
                    A
                  </div>
                  <ChevronDown className="w-3 h-3" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuLabel className="text-xs">
                  <div className="flex flex-col space-y-1">
                    <p className="font-medium">admin@sealbox.dev</p>
                    <p className="text-muted-foreground">{t('nav.administrator')}</p>
                  </div>
                </DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuItem className="text-xs">
                  <Settings className="mr-2 h-3 w-3" />
                  {t('nav.settings')}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem className="text-xs text-red-600">
                  <LogOut className="mr-2 h-3 w-3" />
                  {t('nav.signOut')}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>
    </nav>
  );
}
