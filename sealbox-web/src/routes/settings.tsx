"use client";

import { createFileRoute } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { PageContainer } from "@/components/layout/page-container";
import { PageLayout } from "@/components/layout/page-layout";
import { ContentCard } from "@/components/common/content-card";
import { useServerStatus } from "@/hooks/useServerStatus";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Copy } from "lucide-react";
import { useAuthStore } from "@/stores/auth";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

function SettingsPage() {
  return (
    <PageContainer>
      <Content />
    </PageContainer>
  );
}

function Content() {
  const { t } = useTranslation();
  const status = useServerStatus();
  const { serverUrl } = useAuthStore();

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch {}
  };

  return (
    <PageLayout title={t("pages.settings.title")} subtitle={t("pages.settings.subtitle")}>
      <div className="grid gap-4 grid-cols-2">
        <ContentCard className="space-y-3">
          <div className="text-sm font-medium">{t("pages.settings.server")}</div>
          <div className="text-sm text-muted-foreground flex items-center gap-2">
            <span className="truncate">{serverUrl || t("pages.settings.notConnected")}</span>
            {serverUrl && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                onClick={() => copy(serverUrl)}
              >
                <Copy className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
          <div className="text-xs text-muted-foreground">
            Status:{" "}
            <Badge variant="secondary" className="align-middle">
              {status.status}
            </Badge>
            {status.responseTime ? (
              <span className="ml-2">{status.responseTime} ms</span>
            ) : null}
          </div>
          <div>
            <Button
              variant="outline"
              size="sm"
              onClick={status.refresh}
              className="mt-2"
            >
              Refresh
            </Button>
          </div>
        </ContentCard>

        <ContentCard className="space-y-3">
          <div className="text-sm font-medium">CLI Quickstart</div>
          <div className="text-xs text-muted-foreground">
            Bring a client online and store your first secret
          </div>
          <div className="space-y-2">
            <code className="block bg-muted px-2 py-1 rounded text-xs font-mono">
              sealbox-cli up --name "my-laptop" --enroll
            </code>
            <code className="block bg-muted px-2 py-1 rounded text-xs font-mono">
              sealbox-cli secret set demo "hello" --clients my-laptop
            </code>
          </div>
        </ContentCard>
      </div>
    </PageLayout>
  );
}
