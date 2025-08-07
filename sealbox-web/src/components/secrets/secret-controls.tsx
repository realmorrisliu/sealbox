import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Plus, Search, TableIcon, LayoutGrid, RefreshCw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CreateSecretDialog } from "./create-secret-dialog";
import { useQueryClient } from "@tanstack/react-query";
import { useCleanupExpiredSecrets } from "@/hooks/use-api";
import { toast } from "sonner";
import { useState } from "react";

interface SecretControlsProps {
  searchTerm: string;
  onSearchChange: (term: string) => void;
  viewMode: "table" | "cards";
  onViewModeChange: (mode: "table" | "cards") => void;
}

export function SecretControls({
  searchTerm,
  onSearchChange,
  viewMode,
  onViewModeChange,
}: SecretControlsProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const cleanupMutation = useCleanupExpiredSecrets();
  const [isRefreshing, setIsRefreshing] = useState(false);

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await queryClient.invalidateQueries({ queryKey: ["secrets"] });
      toast.success(t("secrets.refreshed"));
    } catch (error) {
      toast.error(t("common.error"));
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleCleanup = async () => {
    try {
      const result = await cleanupMutation.mutateAsync();
      if (result.deleted_count > 0) {
        toast.success(t("secrets.cleanupSuccess"), {
          description: t("secrets.cleanupSuccessDescription", { count: result.deleted_count }),
        });
      } else {
        toast.info(t("secrets.cleanupNoExpired"));
      }
    } catch (error: any) {
      toast.error(t("common.error"), {
        description: error.message || "Failed to cleanup expired secrets",
      });
    }
  };

  return (
    <Card className="p-3">
      <div className="flex flex-col md:flex-row gap-3 items-start md:items-center justify-between">
        <div className="flex flex-col sm:flex-row gap-2 flex-1">
          <div className="relative">
            <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-3 w-3 text-muted-foreground" />
            <Input
              placeholder={t("secrets.controls.searchPlaceholder")}
              value={searchTerm}
              onChange={(e) => onSearchChange(e.target.value)}
              className="pl-7 h-8 w-64"
            />
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              className="h-8"
              onClick={handleRefresh}
              disabled={isRefreshing}
              title={t("secrets.controls.refresh")}
            >
              <RefreshCw className={`h-3 w-3 mr-1 ${isRefreshing ? "animate-spin" : ""}`} />
              {t("common.refresh")}
            </Button>

            <Button
              variant="outline"
              size="sm"
              className="h-8"
              onClick={handleCleanup}
              disabled={cleanupMutation.isPending}
              title={t("secrets.controls.cleanup")}
            >
              <Trash2 className="h-3 w-3 mr-1" />
              {t("secrets.controls.cleanup")}
            </Button>

            <CreateSecretDialog>
              <Button size="sm" className="h-8">
                <Plus className="h-3 w-3 mr-1" />
                {t("secrets.controls.addSecret")}
              </Button>
            </CreateSecretDialog>
          </div>
        </div>

        <Tabs value={viewMode} onValueChange={onViewModeChange}>
          <TabsList className="h-8">
            <TabsTrigger
              value="table"
              className="px-2"
              title={t("secrets.controls.table")}
            >
              <TableIcon className="h-4 w-4" />
            </TabsTrigger>
            <TabsTrigger
              value="cards"
              className="px-2"
              title={t("secrets.controls.cards")}
            >
              <LayoutGrid className="h-4 w-4" />
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>
    </Card>
  );
}
