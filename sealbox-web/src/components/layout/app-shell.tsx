"use client";

import { Navigation } from "@/components/layout/navigation";

interface AppShellProps {
  children: React.ReactNode;
}

// Tailscale-inspired application shell: slim topbar, tabbed nav,
// generous whitespace, and a centered content container.
export function AppShell({ children }: AppShellProps) {
  return (
    <div className="min-h-screen w-full bg-muted/40">
      <Navigation />
      <main className="container mx-auto max-w-screen-2xl px-8 md:px-10 lg:px-12 py-8 md:py-10 lg:py-12">
        {children}
      </main>
    </div>
  );
}
