import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Eye, Trash2, Package, Shield } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CountdownTimer } from "@/components/common/countdown-timer";
import { EmptyState } from "@/components/common/empty-state";
import type { SecretUIData } from "@/lib/types";
import { useState } from "react";
import { PermissionsDialog } from "@/components/secrets/permissions-dialog";

interface SecretTableProps {
  secrets: SecretUIData[];
  onShowDecryptHint: (secretName: string) => void;
  onDeleteSecret: (secret: SecretUIData) => void;
  isDeleting: boolean;
}

export function SecretTable({
  secrets,
  onShowDecryptHint,
  onDeleteSecret,
  isDeleting,
}: SecretTableProps) {
  const { t } = useTranslation();
  const [permOpenFor, setPermOpenFor] = useState<string | null>(null);

  if (secrets.length === 0) {
    return (
      <EmptyState
        icon={Package}
        title={t("secrets.empty.title")}
        description={t("secrets.empty.description")}
      />
    );
  }

  return (
    <div className="overflow-hidden rounded-xl border bg-background">
      <Table>
        <TableHeader>
          <TableRow className="text-xs border-b bg-muted/30">
            <TableHead className="h-10 px-3 font-semibold">
              {t("secrets.table.name")}
            </TableHead>
            <TableHead className="h-10 px-3 font-semibold">
              {t("secrets.table.status")}
            </TableHead>
            <TableHead className="h-10 px-3 font-semibold">
              {t("secrets.table.created")}
            </TableHead>
            <TableHead className="h-10 px-3 font-semibold">
              {t("secrets.table.updated")}
            </TableHead>
            <TableHead className="h-10 px-3 font-semibold">
              {t("secrets.table.ttl")}
            </TableHead>
            <TableHead className="w-40 h-10 px-3 font-semibold">
              {t("secrets.actions")}
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {secrets.map((secret) => (
            <TableRow
              key={`${secret.key}-${secret.version}`}
              className="text-xs min-h-12 border-b border-border hover:bg-accent/50 transition-colors duration-150"
            >
              <TableCell className="px-3 py-3 font-medium">
                <div className="flex items-center gap-2">
                  <span className="font-mono">{secret.key}</span>
                  <span className="text-xs text-muted-foreground">
                    v{secret.version}
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 w-6 p-0"
                    onClick={() => onShowDecryptHint(secret.key)}
                    title={t("secrets.decryptHint.tooltip")}
                  >
                    <Eye className="h-3 w-3" />
                  </Button>
                </div>
              </TableCell>
              <TableCell className="px-3 py-3">
                <Badge
                  variant={
                    secret.status === "active"
                      ? "success"
                      : secret.status === "expiring"
                        ? "warning"
                        : "destructive"
                  }
                  className="text-xs px-1.5 py-0.5"
                >
                  {t(`secrets.status.${secret.status}`)}
                </Badge>
              </TableCell>
              <TableCell className="px-3 py-3 text-muted-foreground">
                {secret.createdAt}
              </TableCell>
              <TableCell className="px-3 py-3 text-muted-foreground">
                {secret.updatedAt}
              </TableCell>
              <TableCell className="px-3 py-3">
                {secret.expires_at ? (
                  <CountdownTimer
                    expiresAt={secret.expires_at}
                    className="text-xs"
                  />
                ) : (
                  <span className="text-xs text-muted-foreground">
                    {t("common.noExpiry")}
                  </span>
                )}
              </TableCell>
              <TableCell className="px-3 py-3">
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-2"
                    onClick={() => setPermOpenFor(secret.key)}
                    title={t("secrets.permissions.clients")}
                  >
                    <Shield className="h-3 w-3 mr-1" />
                    <span className="inline">
                      {t("secrets.permissions.clients")}
                    </span>
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 w-6 p-0 text-red-600"
                    onClick={() => onDeleteSecret(secret)}
                    disabled={isDeleting}
                    title={t("secrets.confirmDelete", { name: secret.key })}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {permOpenFor && (
        <PermissionsDialog
          secretKey={permOpenFor}
          open={!!permOpenFor}
          onOpenChange={(o) => !o && setPermOpenFor(null)}
        />
      )}
    </div>
  );
}
