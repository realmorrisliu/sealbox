import { Link, useRouter } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { SealboxLogo } from "@/components/ui/sealbox-logo";
import { UserMenu } from "@/components/ui/user-menu";
import { useAuthStore } from "@/stores/auth";

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const router = useRouter();
  const { t } = useTranslation();
  
  // Get current page title based on route
  const getCurrentPageTitle = () => {
    const pathname = router.state.location.pathname;
    if (pathname === '/') return t('nav.secretManagement');
    if (pathname.startsWith('/keys')) return t('nav.masterKeys');
    if (pathname.startsWith('/admin')) return t('nav.administration');
    return t('nav.secretManagement');
  };

  return (
    <div className="bg-background min-h-screen">
      {/* Linear-style minimal navigation - 48px height */}
      <header className="sticky top-0 z-50 bg-card border-b border-border">
        <div className="container-main h-12 flex items-center justify-between">
          {/* Left: Logo + Current Page */}
          <div className="flex items-center space-x-6">
            <Link to="/" className="group flex items-center">
              <SealboxLogo size="sm" className="group-hover:translate-y-[-1px] transition-transform duration-150" />
            </Link>
            
            {/* Current Page Title - Always visible on mobile */}
            <div className="block">
              <h1 className="text-sm font-medium text-foreground truncate max-w-[200px] sm:max-w-none">
                {getCurrentPageTitle()}
              </h1>
            </div>
          </div>

          {/* Right: User Menu */}
          <UserMenu />
        </div>
      </header>

      {/* Main content area - Enhanced spacing */}
      <main className="container-main py-16">
        {children}
      </main>
    </div>
  );
}