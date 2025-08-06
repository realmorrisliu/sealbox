import { useState, useMemo } from "react";
import { filterSecrets, calculateSecretStats } from "@/lib/secret-utils";
import type { SecretUIData } from "@/lib/types";

export function useSecretFiltering(secrets: SecretUIData[]) {
  const [searchTerm, setSearchTerm] = useState("");

  const filteredSecrets = useMemo(() => {
    return filterSecrets(secrets, searchTerm);
  }, [secrets, searchTerm]);

  const stats = useMemo(() => {
    return calculateSecretStats(secrets);
  }, [secrets]);

  return {
    searchTerm,
    setSearchTerm,
    filteredSecrets,
    stats,
  };
}
