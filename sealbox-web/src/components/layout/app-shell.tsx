"use client";

import { Navigation } from "@/components/layout/navigation";

interface AppShellProps {
  children: React.ReactNode;
}

// Tailscale-inspired application shell: slim topbar, tabbed nav,
// generous whitespace, and a centered content container.
export function AppShell({ children }: AppShellProps) {
  return (
    <div className="min-h-screen w-full bg-white">
      <Navigation />
      <main className="container mx-auto max-w-screen-2xl px-16 py-8 bg-white">
        {children}
      </main>
    </div>
  );
}
