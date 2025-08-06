import { Card } from "@/components/ui/card";
import { Key, AlertTriangle, Clock } from "lucide-react";
import { useTranslation } from "react-i18next";

interface SecretStatsProps {
  stats: {
    total: number;
    expiring: number;
    expired: number;
  };
}

export function SecretStats({ stats }: SecretStatsProps) {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-3 gap-2">
      <Card className="p-3">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-xs text-muted-foreground">
              {t("secrets.stats.totalSecrets")}
            </p>
            <p className="text-lg font-bold">{stats.total}</p>
          </div>
          <Key className="w-4 h-4 text-blue-500" />
        </div>
      </Card>

      <Card className="p-3">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-xs text-muted-foreground">
              {t("secrets.stats.expiring")}
            </p>
            <p className="text-lg font-bold">{stats.expiring}</p>
          </div>
          <AlertTriangle className="w-4 h-4 text-yellow-500" />
        </div>
      </Card>

      <Card className="p-3">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-xs text-muted-foreground">
              {t("secrets.stats.expired")}
            </p>
            <p className="text-lg font-bold">{stats.expired}</p>
          </div>
          <Clock className="w-4 h-4 text-red-500" />
        </div>
      </Card>
    </div>
  );
}
