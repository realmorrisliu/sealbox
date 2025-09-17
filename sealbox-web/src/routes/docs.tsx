"use client";

import { createFileRoute } from "@tanstack/react-router";
import { PageContainer } from "@/components/layout/page-container";

export const Route = createFileRoute("/docs")({
  component: DocsPage,
});

function DocsPage() {
  return (
    <PageContainer>
      <div className="p-6 text-sm text-muted-foreground">Documentation coming soon.</div>
    </PageContainer>
  );
}

