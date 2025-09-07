"use client";

import { Button } from "@/components/ui/button";
import { Link, useRouterState } from "@tanstack/react-router";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
// We inline language and theme controls inside the user menu.
import { LogOut, ChevronDown, Globe, Sun, Moon, Monitor } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAuthStore } from "@/stores/auth";
import { cn } from "@/lib/utils";
import { useTheme } from "@/components/theme/theme-provider";
import {
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from "@/components/ui/dropdown-menu";

import { AdaptiveLogo } from "@/components/brand/adaptive-logo";

// Two-row Tailscale-like navigation
export function Navigation() {
  const { t } = useTranslation();
  const { logout } = useAuthStore();

  return (
    <nav className="sticky top-0 z-50 w-full bg-muted/95 backdrop-blur">
      {/* Top bar */}
      <div className="border-b border-border">
        <div className="container mx-auto max-w-screen-2xl h-20 px-10 md:px-12 lg:px-16 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <AdaptiveLogo size={28} />
            <div className="leading-tight">
              <div className="font-semibold">Sealbox</div>
              <div className="text-xs text-muted-foreground">
                Self-hosted secrets hub
              </div>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <Link
              to="/docs"
              className="text-sm text-foreground/80 hover:text-primary px-4 py-2.5"
            >
              Docs
            </Link>
            <a
              href="https://github.com/"
              target="_blank"
              rel="noreferrer"
              className="text-sm text-foreground/80 hover:text-primary px-4 py-2.5"
            >
              Support
            </a>
            <UserMenu onLogout={logout} label={t("auth.logout")} />
          </div>
        </div>
      </div>

      {/* Sub navigation */}
      <div className="border-b border-border">
        <div className="container mx-auto max-w-screen-2xl px-10 md:px-12 lg:px-16">
          <MainTabs />
        </div>
      </div>
    </nav>
  );
}

function UserMenu({
  onLogout,
  label,
}: {
  onLogout: () => void;
  label: string;
}) {
  const { theme, setTheme } = useTheme();
  const { i18n } = useTranslation();
  const languages = [
    { code: "en", name: "English", icon: "🇺🇸" },
    { code: "zh", name: "中文", icon: "🇨🇳" },
    { code: "ja", name: "日本語", icon: "🇯🇵" },
    { code: "de", name: "Deutsch", icon: "🇩🇪" },
  ];
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="h-9 px-2">
          <div className="h-6 w-6 rounded-full bg-muted" />
          <ChevronDown className="ml-1 h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuLabel>Account</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Sun className="h-4 w-4" />
            Appearance
          </DropdownMenuSubTrigger>
          <DropdownMenuSubContent className="w-44">
            <DropdownMenuRadioGroup
              value={theme}
              onValueChange={setTheme as any}
            >
              <DropdownMenuRadioItem value="light">
                <Sun className="h-4 w-4" /> Light
              </DropdownMenuRadioItem>
              <DropdownMenuRadioItem value="dark">
                <Moon className="h-4 w-4" /> Dark
              </DropdownMenuRadioItem>
              <DropdownMenuRadioItem value="system">
                <Monitor className="h-4 w-4" /> System
              </DropdownMenuRadioItem>
            </DropdownMenuRadioGroup>
          </DropdownMenuSubContent>
        </DropdownMenuSub>
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>
            <Globe className="h-4 w-4" />
            Language
          </DropdownMenuSubTrigger>
          <DropdownMenuSubContent className="w-44">
            <DropdownMenuRadioGroup
              value={i18n.language}
              onValueChange={(v) => {
                i18n.changeLanguage(v);
                if (typeof window !== "undefined") {
                  localStorage.setItem("sealbox-language", v);
                }
              }}
            >
              {languages.map((l) => (
                <DropdownMenuRadioItem key={l.code} value={l.code}>
                  <span className="mr-2">{l.icon}</span>
                  {l.name}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuSubContent>
        </DropdownMenuSub>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={onLogout} className="text-red-600">
          <LogOut className="mr-2 h-4 w-4" /> {label}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function MainTabs() {
  const router = useRouterState();
  const links = [
    { to: "/", label: "Secrets" },
    { to: "/clients", label: "Clients" },
    { to: "/settings", label: "Settings" },
  ];

  const activePath = router.location.pathname;
  return (
    <div className="flex items-center gap-3 h-14">
      {links.map((l) => {
        const active =
          activePath === l.to || (l.to !== "/" && activePath.startsWith(l.to));
        return (
          <Link
            key={l.to}
            to={l.to}
            className={cn(
              "rounded-md px-4 py-2.5 text-sm transition-all duration-200",
              active
                ? "bg-background text-foreground shadow-sm border"
                : "text-foreground/70 hover:text-primary hover:bg-background/50",
            )}
          >
            {l.label}
          </Link>
        );
      })}
    </div>
  );
}
