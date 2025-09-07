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
import { LogOut, ChevronDown, User } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAuthStore } from "@/stores/auth";
import { cn } from "@/lib/utils";

import { AdaptiveLogo } from "@/components/brand/adaptive-logo";
import { ThemeSelector } from "./theme-selector";
import { LanguageSelector } from "./language-selector";

// Two-row Tailscale-like navigation
export function Navigation() {
  const { t } = useTranslation();
  const { logout } = useAuthStore();

  return (
    <nav className="sticky top-0 z-50 w-full bg-muted/95 backdrop-blur">
      {/* Top bar */}
      <div className="border-b border-border">
        <div className="container mx-auto max-w-screen-2xl h-20 px-16 flex items-center justify-between">
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
            <UserMenu onLogout={logout} label={t("nav.signOut")} />
          </div>
        </div>
      </div>

      {/* Sub navigation */}
      <div className="border-b border-border">
        <div className="container mx-auto max-w-screen-2xl px-14">
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
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="h-9 px-2 hover:!bg-background cursor-pointer"
        >
          <div className="h-6 w-6 rounded-full bg-muted border flex items-center justify-center">
            <User className="h-3.5 w-3.5 text-muted-foreground" />
          </div>
          <ChevronDown className="ml-1 h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-fit">
        <ThemeSelector />
        <LanguageSelector />
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={onLogout} className="cursor-pointer">{label}</DropdownMenuItem>
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
    <div className="flex items-center gap-3 h-10">
      {links.map((l) => {
        const active =
          activePath === l.to || (l.to !== "/" && activePath.startsWith(l.to));
        return (
          <Link
            key={l.to}
            to={l.to}
            className={cn(
              "rounded px-2 py-1 text-sm transition-all duration-200 relative",
              active
                ? "text-primary after:absolute after:-bottom-1.5 after:left-1/2 after:-translate-x-1/2 after:h-0.5 after:bg-primary after:w-[calc(100%-1rem)]"
                : "text-muted-foreground hover:text-primary hover:bg-background",
            )}
          >
            {l.label}
          </Link>
        );
      })}
    </div>
  );
}
