import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Plus, Search, TableIcon, LayoutGrid } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CreateSecretDialog } from "./create-secret-dialog";

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
        </div>

        <div className="flex items-center gap-2">
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

          <CreateSecretDialog>
            <Button size="sm" className="h-8">
              <Plus className="h-3 w-3 mr-1" />
              {t("secrets.controls.addSecret")}
            </Button>
          </CreateSecretDialog>
        </div>
      </div>
    </Card>
  );
}
