// src/routes/__root.tsx
/// <reference types="vite/client" />
import type { ReactNode } from "react";
import {
  Outlet,
  createRootRoute,
  HeadContent,
  Scripts,
} from "@tanstack/react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { useSSRSafeTranslation } from "@/hooks/useSSRSafeTranslation";

import appCss from "@/styles/app.css?url";
import { queryClient } from "@/lib/query-client";
import { Toaster } from "@/components/ui/sonner";
import { ErrorBoundary } from "@/components/error-boundary";
import { ThemeProvider } from "@/components/theme/theme-provider";
import { HtmlAttributes } from "@/components/common/html-attributes";
import "@/lib/i18n";

function NotFound() {
  const { t } = useSSRSafeTranslation();

  return (
    <div className="min-h-screen flex items-center justify-center bg-background">
      <div className="text-center space-y-4">
        <h1 className="text-4xl font-bold text-foreground">404</h1>
        <p className="text-muted-foreground">{t("errors.pageNotFound")}</p>
        <p className="text-sm text-muted-foreground">
          {t("errors.pageNotFoundDescription")}
        </p>
      </div>
    </div>
  );
}

export const Route = createRootRoute({
  head: () => ({
    meta: [
      {
        charSet: "utf-8",
      },
      {
        name: "viewport",
        content: "width=device-width, initial-scale=1",
      },
      {
        title: "Sealbox - Secret Management System",
      },
    ],
    links: [
      {
        rel: "stylesheet",
        href: appCss,
      },
    ],
  }),
  component: RootComponent,
  notFoundComponent: NotFound,
});

function RootComponent() {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider>
          <HtmlAttributes />
          <RootDocument>
            <Outlet />
            <Toaster />
            <ReactQueryDevtools initialIsOpen={false} />
          </RootDocument>
        </ThemeProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html>
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  );
}
