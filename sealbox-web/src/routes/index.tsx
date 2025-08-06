"use client";

import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { AuthGuard } from "@/components/auth/auth-guard";
import { MainLayout } from "@/components/layout/main-layout";
import { SecretStats } from "@/components/secrets/secret-stats";
import { SecretControls } from "@/components/secrets/secret-controls";
import { SecretTable } from "@/components/secrets/secret-table";
import { SecretCards } from "@/components/secrets/secret-cards";
import { useSecretManagement } from "@/hooks/useSecretManagement";
import { useSecretFiltering } from "@/hooks/useSecretFiltering";
import { useTranslation } from "react-i18next";

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
  const [viewMode, setViewMode] = useState<"table" | "cards">("table");

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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        {t("common.loading")}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center p-8 text-red-500">
        {t("common.error")}
      </div>
    );
  }

  return (
    <div className="w-full px-2 py-4 space-y-4">
      {/* Stats Cards */}
      <SecretStats stats={stats} />

      {/* Controls */}
      <SecretControls
        searchTerm={searchTerm}
        onSearchChange={setSearchTerm}
        viewMode={viewMode}
        onViewModeChange={setViewMode}
      />

      {/* Secrets Display */}
      {viewMode === "table" ? (
        <SecretTable
          secrets={filteredSecrets}
          onShowDecryptHint={showDecryptHint}
          onDeleteSecret={handleDeleteSecret}
          isDeleting={isDeleting}
        />
      ) : (
        <SecretCards
          secrets={filteredSecrets}
          onShowDecryptHint={showDecryptHint}
          onDeleteSecret={handleDeleteSecret}
          isDeleting={isDeleting}
        />
      )}
    </div>
  );
}
