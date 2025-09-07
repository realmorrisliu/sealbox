import { AppShell } from "@/components/layout/app-shell";

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  return <AppShell>{children}</AppShell>;
}
