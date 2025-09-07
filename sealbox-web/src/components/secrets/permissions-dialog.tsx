"use client";

import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Alert } from "@/components/ui/alert";
import { AlertTriangle, Shield } from "lucide-react";
import {
  useSecretPermissions,
  useRevokeSecretPermission,
} from "@/hooks/use-api";

export function PermissionsDialog({
  secretKey,
  open,
  onOpenChange,
}: {
  secretKey: string;
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const { t } = useTranslation();
  const { data, isLoading, error } = useSecretPermissions(secretKey, open);
  const revoke = useRevokeSecretPermission();
  const [confirming, setConfirming] = useState<string | null>(null);

  const items = data?.authorized_clients || [];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {t("secrets.permissions.title", { key: secretKey })}
          </DialogTitle>
          <DialogDescription>
            {t("secrets.permissions.description")}
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="text-sm text-muted-foreground">
            {t("common.loading")}
          </div>
        ) : error ? (
          <Alert variant="destructive">
            <AlertTriangle className="h-4 w-4" />
            <div>
              <p className="font-medium">{t("common.error")}</p>
              <p className="text-sm">{(error as any).message}</p>
            </div>
          </Alert>
        ) : items.length === 0 ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Shield className="h-4 w-4" />
            {t("secrets.permissions.noPermissions")}
          </div>
        ) : (
          <ScrollArea className="max-h-64 pr-2">
            <ul className="space-y-2">
              {items.map((it) => (
                <li
                  key={it.client_id}
                  className="flex items-center justify-between gap-2"
                >
                  <div className="min-w-0">
                    <div className="text-sm font-medium truncate">
                      {it.client_name || `${it.client_id.substring(0, 12)}...`}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {t("secrets.permissions.authorizedAt")}:{" "}
                      {new Date(it.authorized_at * 1000).toLocaleString()}
                    </div>
                  </div>
                  {confirming === it.client_id ? (
                    <div className="flex items-center gap-2">
                      <Button
                        size="sm"
                        variant="destructive"
                        onClick={async () => {
                          await revoke.mutateAsync({
                            key: secretKey,
                            clientId: it.client_id,
                          });
                          setConfirming(null);
                        }}
                        disabled={revoke.isPending}
                      >
                        {t("secrets.permissions.revoke")}
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => setConfirming(null)}
                      >
                        {t("common.cancel")}
                      </Button>
                    </div>
                  ) : (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => setConfirming(it.client_id)}
                    >
                      {t("secrets.permissions.revoke")}
                    </Button>
                  )}
                </li>
              ))}
            </ul>
          </ScrollArea>
        )}

        {items.length > 0 && (
          <div className="text-xs text-muted-foreground mt-2">
            <Badge variant="secondary" className="mr-1">
              {t("secrets.permissions.authorizedClients", {
                count: items.length,
              })}
            </Badge>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
