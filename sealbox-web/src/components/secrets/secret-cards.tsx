import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { Eye, Trash2, Clock } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CountdownTimer } from "@/components/common/countdown-timer";
import { getStatusBadge } from "@/lib/secret-utils";
import type { SecretUIData } from "@/lib/types";

interface SecretCardsProps {
  secrets: SecretUIData[];
  onShowDecryptHint: (secretName: string) => void;
  onDeleteSecret: (secret: SecretUIData) => void;
  isDeleting: boolean;
}

export function SecretCards({
  secrets,
  onShowDecryptHint,
  onDeleteSecret,
  isDeleting,
}: SecretCardsProps) {
  const { t } = useTranslation();

  return (
    <div className="grid gap-3">
      {secrets.map((secret) => (
        <Card key={`${secret.key}-${secret.version}`} className="p-4">
          <div className="flex items-start justify-between">
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-2">
                <h3 className="font-mono font-medium">{secret.key}</h3>
                <Badge
                  className={`text-xs px-1.5 py-0.5 ${getStatusBadge(secret.status)}`}
                >
                  {t(`secrets.status.${secret.status}`)}
                </Badge>
                <span className="text-xs text-muted-foreground">
                  v{secret.version}
                </span>
              </div>

              <div className="flex items-center gap-2 mb-2">
                <code className="bg-muted px-2 py-1 rounded text-xs font-mono flex-1 min-w-0 truncate text-muted-foreground">
                  [ENCRYPTED]
                </code>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 w-7 p-0"
                  onClick={() => onShowDecryptHint(secret.key)}
                  title={t("secrets.decryptHint.tooltip")}
                >
                  <Eye className="h-3 w-3" />
                </Button>
              </div>

              <div className="flex items-center gap-4 text-xs text-muted-foreground">
                <span>
                  {t("secrets.table.created")}: {secret.createdAt}
                </span>
                <span>
                  {t("secrets.table.updated")}: {secret.updatedAt}
                </span>
                {secret.expires_at && (
                  <div className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    <CountdownTimer
                      expiresAt={secret.expires_at}
                      className="text-xs"
                    />
                  </div>
                )}
              </div>
            </div>

            <Button
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0 text-red-600"
              onClick={() => onDeleteSecret(secret)}
              disabled={isDeleting}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </Card>
      ))}
    </div>
  );
}
