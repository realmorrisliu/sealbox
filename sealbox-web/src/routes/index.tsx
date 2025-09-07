"use client";

import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { AuthGuard } from "@/components/auth/auth-guard";
import { MainLayout } from "@/components/layout/main-layout";
import { SecretTable } from "@/components/secrets/secret-table";
import { SecretsListSkeleton } from "@/components/common/loading-skeletons";
import { useSecretManagement } from "@/hooks/useSecretManagement";
import { useSecretFiltering } from "@/hooks/useSecretFiltering";
import { useTranslation } from "react-i18next";
import { Alert } from "@/components/ui/alert";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { CreateSecretDialog } from "@/components/secrets/create-secret-dialog";
import { PageHeader } from "@/components/layout/page-header";
import { Badge } from "@/components/ui/badge";
import { useClientKeys, useApproveEnrollment } from "@/hooks/use-api";
import { Label } from "@/components/ui/label";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <AuthGuard>
      <MainLayout>
        <SecretManagement />
      </MainLayout>
    </AuthGuard>
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

  if (isLoading) {
    return <SecretsListSkeleton />;
  }

  if (error) {
    return (
      <div className="w-full px-2 py-4">
        <Alert variant="destructive">
          <AlertTriangle className="h-4 w-4" />
          <div>
            <p className="font-medium">{t("common.error")}</p>
            <p className="text-sm mt-1">{t("common.errorDescription")}</p>
          </div>
        </Alert>
      </div>
    );
  }

  const noSecrets = (filteredSecrets || []).length === 0;
  const noClients = (clientsData?.client_keys || []).length === 0;

  return (
    <>
      <PageHeader
        title="Secrets"
        subtitle="Manage secrets and client access."
        meta={
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="text-xs">
              {filteredSecrets.length} secrets
            </Badge>
            {searchTerm && (
              <Badge variant="outline" className="text-xs">
                Filtered
              </Badge>
            )}
          </div>
        }
        actions={
          <div className="flex items-center gap-3">
            <div className="hidden md:block">
              <Input
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                placeholder={t("secrets.controls.searchPlaceholder")}
                className="h-10 w-64"
              />
            </div>
            <CreateSecretDialog>
              <Button size="sm" className="h-9 px-4">
                {t("secrets.controls.addSecret")}
              </Button>
            </CreateSecretDialog>
          </div>
        }
      />

      {/* Unified onboarding when nothing exists */}
      {noSecrets && noClients ? (
        <div className="p-6 md:p-7 rounded-xl border bg-background">
          <div className="grid md:grid-cols-2 gap-6 items-start">
            <div className="space-y-2">
              <h3 className="text-lg font-semibold">
                Get started in two steps
              </h3>
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
        </div>
      ) : (
        <SecretTable
          secrets={filteredSecrets}
          onShowDecryptHint={showDecryptHint}
          onDeleteSecret={handleDeleteSecret}
          isDeleting={isDeleting}
        />
      )}
    </>
  );
}
