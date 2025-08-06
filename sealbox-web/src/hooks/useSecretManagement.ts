import { useState, useEffect } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useSecrets, useDeleteSecret } from "./use-api";
import { convertSecretToUIData } from "@/lib/secret-utils";
import type { SecretUIData } from "@/lib/types";

export function useSecretManagement() {
  const { t } = useTranslation();
  const { data: secretsData, isLoading, error } = useSecrets();
  const deleteSecretMutation = useDeleteSecret();
  const [secrets, setSecrets] = useState<SecretUIData[]>([]);

  // Convert API data to display format
  useEffect(() => {
    if (secretsData?.secrets) {
      const convertedSecrets = secretsData.secrets.map(convertSecretToUIData);
      setSecrets(convertedSecrets);
    }
  }, [secretsData]);

  const handleDeleteSecret = async (secret: SecretUIData) => {
    if (!window.confirm(t("secrets.confirmDelete", { name: secret.key }))) {
      return;
    }

    try {
      await deleteSecretMutation.mutateAsync({
        key: secret.key,
        version: secret.version,
      });

      toast.success(t("secrets.deleted"), {
        description: t("secrets.deletedDescription", { name: secret.key }),
      });
    } catch (error: any) {
      toast.error(t("common.error"), {
        description: error.message || "Failed to delete secret",
      });
    }
  };

  const showDecryptHint = (secretName: string) => {
    toast.info(t("secrets.decryptHint.title"), {
      description: t("secrets.decryptHint.description", {
        command: `sealbox-cli secret get ${secretName}`,
      }),
      duration: 5000,
    });
  };

  return {
    secrets,
    isLoading,
    error,
    handleDeleteSecret,
    showDecryptHint,
    isDeleting: deleteSecretMutation.isPending,
  };
}
