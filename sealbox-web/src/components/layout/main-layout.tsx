import { Link, useRouter } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { useAuthStore } from "@/stores/auth";
import { useHealthCheck } from "@/hooks/use-api";
import { KeyRound, Server, Settings, LogOut, Activity } from "lucide-react";

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { logout, serverUrl } = useAuthStore();
  const router = useRouter();
  const { data: healthStatus } = useHealthCheck();

  const handleLogout = () => {
    logout();
    router.navigate({ to: "/login" });
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Top navigation bar */}
      <header className="border-b bg-card">
        <div className="container mx-auto px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-6">
              <Link to="/" className="flex items-center space-x-2">
                <KeyRound className="h-6 w-6" />
                <span className="text-xl font-bold">Sealbox</span>
              </Link>
              
              <nav className="flex items-center space-x-4">
                <Link
                  to="/"
                  className="text-sm font-medium hover:text-primary"
                  activeProps={{ className: "text-primary" }}
                >
                  Secret Management
                </Link>
                {/* Temporarily hide unimplemented pages */}
                {/* 
                <Link
                  to="/keys"
                  className="text-sm font-medium hover:text-primary"
                  activeProps={{ className: "text-primary" }}
                >
                  Master Keys
                </Link>
                <Link
                  to="/admin"
                  className="text-sm font-medium hover:text-primary"
                  activeProps={{ className: "text-primary" }}
                >
                  Administration
                </Link>
                */}
              </nav>
            </div>

            <div className="flex items-center space-x-4">
              {/* Server status */}
              <div className="flex items-center space-x-2 text-sm text-muted-foreground">
                <Activity
                  className={`h-4 w-4 ${
                    healthStatus ? "text-green-500" : "text-red-500"
                  }`}
                />
                <span className="hidden sm:inline">
                  {serverUrl ? new URL(serverUrl).host : "Not connected"}
                </span>
              </div>

              {/* Settings button */}
              <Button variant="ghost" size="icon">
                <Settings className="h-4 w-4" />
              </Button>

              {/* Logout button */}
              <Button variant="ghost" size="icon" onClick={handleLogout}>
                <LogOut className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      </header>

      {/* Main content area */}
      <main className="container mx-auto px-4 py-6">
        {children}
      </main>
    </div>
  );
}