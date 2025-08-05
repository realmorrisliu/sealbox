import { Link, useRouter } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { Shield, Key } from "lucide-react";
import { Button } from "@/components/ui/button";

export function Navigation() {
  const router = useRouter();
  const { t } = useTranslation();
  const pathname = router.state.location.pathname;

  const navItems = [
    {
      to: "/",
      label: t('nav.secretManagement'),
      icon: Shield,
      isActive: pathname === "/"
    },
    {
      to: "/keys",
      label: t('nav.masterKeys'),
      icon: Key,
      isActive: pathname === "/keys"
    }
  ];

  return (
    <nav className="flex items-center space-x-1">
      {navItems.map((item) => {
        const Icon = item.icon;
        return (
          <Link key={item.to} to={item.to}>
            <Button
              variant={item.isActive ? "default" : "ghost"}
              size="sm"
              className="h-8 px-3 text-xs transition-colors duration-150"
            >
              <Icon className="h-3 w-3 mr-1.5" />
              <span className="hidden sm:inline">{item.label}</span>
            </Button>
          </Link>
        );
      })}
    </nav>
  );
}