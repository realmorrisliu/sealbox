import { Navigation } from "@/components/layout/navigation";

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  return (
    <div className="bg-background min-h-screen">
      {/* Navigation */}
      <Navigation />

      {/* Main content area */}
      <main className="w-full px-2 py-4">
        {children}
      </main>
    </div>
  );
}