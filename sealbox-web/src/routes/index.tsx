"use client";

import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { PageContainer } from "@/components/layout/page-container";
import { PageLayout } from "@/components/layout/page-layout";
import { DataSection } from "@/components/common/data-section";
import { EmptyState } from "@/components/common/empty-state";
import { SecretTable } from "@/components/secrets/secret-table";
import { SecretsListSkeleton } from "@/components/common/loading-skeletons";
import { useSecretManagement } from "@/hooks/useSecretManagement";
import { useSecretFiltering } from "@/hooks/useSecretFiltering";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { CreateSecretDialog } from "@/components/secrets/create-secret-dialog";
import { useClientKeys, useApproveEnrollment } from "@/hooks/use-api";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <PageContainer>
      <SecretManagement />
    </PageContainer>
  );
}

function SecretManagement() {
  const { t } = useTranslation();

  // Business logic hooks
  const {
    secrets,
    isLoading,
    error,
    handleDeleteSecret,
    showDecryptHint,
    isDeleting,
  } = useSecretManagement();

  const { searchTerm, setSearchTerm, filteredSecrets, stats } =
    useSecretFiltering(secrets);

  const { data: clientsData } = useClientKeys();
  const approveEnrollment = useApproveEnrollment();
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");

  const noSecrets = (filteredSecrets || []).length === 0;
  const noClients = (clientsData?.client_keys || []).length === 0;

  return (
    <PageLayout
      title="Secrets"
      subtitle="Manage secrets and client access."
      stats={{
        count: filteredSecrets.length,
        label: "secrets",
        filtered: !!searchTerm,
      }}
      searchProps={{
        value: searchTerm,
        onChange: setSearchTerm,
        placeholder: t("secrets.controls.searchPlaceholder"),
        size: "md",
      }}
      actions={
        <CreateSecretDialog>
          <Button size="sm" className="h-9 px-4">
            {t("secrets.controls.addSecret")}
          </Button>
        </CreateSecretDialog>
      }
    >
      <DataSection
        loading={isLoading}
        error={error}
        empty={noSecrets && noClients}
        loadingSkeleton={<SecretsListSkeleton />}
        emptyState={
          <EmptyState
            title="Get started in two steps"
            centered={false}
            withContainer={true}
          >
            <div className="grid grid-cols-2 gap-6 items-start">
              <div className="space-y-2">
                <ol className="text-sm text-muted-foreground list-decimal list-inside space-y-1">
                  <li>
                    On your machine run:{" "}
                    <code className="bg-muted px-1 py-0.5 rounded">
                      sealbox-cli up --enroll
                    </code>
                  </li>
                  <li>Approve the code here, then create your first secret</li>
                </ol>
              </div>
              <div className="space-y-2">
                <Label className="text-xs" htmlFor="code">
                  Enrollment code
                </Label>
                <Input
                  id="code"
                  value={code}
                  onChange={(e) => setCode(e.target.value.toUpperCase())}
                  placeholder="ABCD-EFGH"
                  className="h-8"
                />
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <Label className="text-xs" htmlFor="name">
                      Name
                    </Label>
                    <Input
                      id="name"
                      value={name}
                      onChange={(e) => setName(e.target.value)}
                      placeholder="e.g., laptop"
                      className="h-8"
                    />
                  </div>
                  <div>
                    <Label className="text-xs" htmlFor="desc">
                      Description
                    </Label>
                    <Input
                      id="desc"
                      value={desc}
                      onChange={(e) => setDesc(e.target.value)}
                      placeholder="Owner or purpose"
                      className="h-8"
                    />
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    onClick={async () => {
                      if (!code.trim()) return;
                      await approveEnrollment.mutateAsync({
                        code: code.trim(),
                        name: name || undefined,
                        description: desc || undefined,
                      });
                      setCode("");
                    }}
                    disabled={approveEnrollment.isPending || !code.trim()}
                  >
                    Approve
                  </Button>
                  <CreateSecretDialog>
                    <Button variant="outline" size="sm">
                      {t("secrets.controls.addSecret")}
                    </Button>
                  </CreateSecretDialog>
                </div>
              </div>
            </div>
          </EmptyState>
        }
        errorProps={{
          title: "Failed to load secrets",
          description: error?.message,
          onRetry: () => window.location.reload(),
          retryLabel: "Retry",
        }}
      >
        <SecretTable
          secrets={filteredSecrets}
          onShowDecryptHint={showDecryptHint}
          onDeleteSecret={handleDeleteSecret}
          isDeleting={isDeleting}
        />
      </DataSection>
    </PageLayout>
  );
}
