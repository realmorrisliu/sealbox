"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Plus, Loader2, Users, X, ExternalLink } from "lucide-react";
import { Link } from "@tanstack/react-router";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useCreateSecret, useClientKeys } from "@/hooks/use-api";
import type { CreateSecretRequest } from "@/lib/types";

interface CreateSecretDialogProps {
  children?: React.ReactNode;
}

export function CreateSecretDialog({ children }: CreateSecretDialogProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [newSecret, setNewSecret] = useState({
    name: "",
    value: "",
    ttl: "",
  });
  const [selectedClients, setSelectedClients] = useState<string[]>([]);
  const createSecretMutation = useCreateSecret();
  const { data: clientKeysData } = useClientKeys();

  const handleAddSecret = async () => {
    if (!newSecret.name || !newSecret.value) {
      toast.error(t("secrets.dialog.missingFields"), {
        description: t("secrets.dialog.nameAndValueRequired"),
      });
      return;
    }

    // Ensure at least one client selected
    if (selectedClients.length === 0) {
      toast.error(t("secrets.dialog.multiClient.noClientsSelected"), {
        description: t("secrets.dialog.multiClient.selectAtLeastOne"),
      });
      return;
    }

    try {
      const requestData: CreateSecretRequest = {
        secret: newSecret.value,
        ...(newSecret.ttl && { ttl: parseInt(newSecret.ttl, 10) }),
        ...(selectedClients.length > 0 && {
          authorized_clients: selectedClients,
        }),
      };

      await createSecretMutation.mutateAsync({
        key: newSecret.name,
        data: requestData,
      });

      const successMessage = isMultiClient
        ? t("secrets.dialog.multiClient.secretCreated", {
            name: newSecret.name,
            count: selectedClients.length,
          })
        : t("secrets.dialog.hasBeenCreated", {
            name: newSecret.name,
          });

      toast.success(t("secrets.dialog.secretAdded"), {
        description: successMessage,
      });

      setNewSecret({ name: "", value: "", ttl: "" });
      setSelectedClients([]);
      setOpen(false);
    } catch (error: any) {
      toast.error(t("common.error"), {
        description: error.message || "Failed to create secret",
      });
    }
  };

  const handleCancel = () => {
    setNewSecret({ name: "", value: "", ttl: "" });
    setSelectedClients([]);
    setOpen(false);
  };

  const handleClientToggle = (clientId: string, checked: boolean) => {
    if (checked) {
      setSelectedClients((prev) => [...prev, clientId]);
    } else {
      setSelectedClients((prev) => prev.filter((id) => id !== clientId));
    }
  };

  const removeSelectedClient = (clientId: string) => {
    setSelectedClients((prev) => prev.filter((id) => id !== clientId));
  };

  const clientKeys = clientKeysData?.client_keys || [];
  const activeClientKeys = clientKeys.filter((key) => key.status === "Active");

  // When enabling multi-client mode, preselect recently online clients to reduce friction
  useEffect(() => {
    if (selectedClients.length > 0) return;
    const now = Math.floor(Date.now() / 1000);
    const recent = activeClientKeys
      .filter(
        (k: any) =>
          typeof k.last_used_at === "number" &&
          now - k.last_used_at! < 24 * 3600,
      )
      .map((k: any) => k.id);
    if (recent.length > 0) {
      setSelectedClients(recent);
    } else if (activeClientKeys.length > 0) {
      // Fallback to first active client
      setSelectedClients([activeClientKeys[0].id]);
    }
  }, [activeClientKeys.length]);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="max-w-lg max-h-[80vh]">
        <DialogHeader>
          <DialogTitle className="text-lg">
            {t("secrets.dialog.addNewSecret")}
          </DialogTitle>
          <DialogDescription className="text-sm">
            {t("secrets.dialog.addNewSecretDescription")}
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className="max-h-[60vh]">
          <div className="space-y-4 pr-4">
            <div>
              <Label htmlFor="name" className="text-xs">
                {t("secrets.dialog.name")}
              </Label>
              <Input
                id="name"
                placeholder={t("secrets.dialog.nameHelp")}
                value={newSecret.name}
                onChange={(e) =>
                  setNewSecret({ ...newSecret, name: e.target.value })
                }
                className="h-8"
              />
            </div>
            <div>
              <Label htmlFor="value" className="text-xs">
                {t("secrets.dialog.value")}
              </Label>
              <Textarea
                id="value"
                placeholder={t("secrets.dialog.valueHelp")}
                value={newSecret.value}
                onChange={(e) =>
                  setNewSecret({ ...newSecret, value: e.target.value })
                }
                className="min-h-16"
              />
            </div>
            <div>
              <Label htmlFor="ttl" className="text-xs">
                {t("secrets.dialog.ttl")}
              </Label>
              <Input
                id="ttl"
                type="number"
                placeholder={t("secrets.dialog.ttlHelp")}
                value={newSecret.ttl}
                onChange={(e) =>
                  setNewSecret({ ...newSecret, ttl: e.target.value })
                }
                className="h-8"
              />
            </div>

            {/* Clients selection */}
            <div className="border-t pt-4">
              <Label className="text-xs">
                {t("secrets.dialog.multiClient.selectClients")}
              </Label>

              {/* Selected clients badges */}
              {selectedClients.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-2 mb-3">
                  {selectedClients.map((clientId) => {
                    const client = clientKeys.find((k) => k.id === clientId);
                    return (
                      <Badge
                        key={clientId}
                        variant="secondary"
                        className="text-xs"
                      >
                        {client?.name ||
                          `${(client?.id || clientId).substring(0, 8)}...`}
                        <button
                          onClick={() => removeSelectedClient(clientId)}
                          className="ml-1 hover:text-destructive"
                        >
                          <X className="h-3 w-3" />
                        </button>
                      </Badge>
                    );
                  })}
                </div>
              )}

              {/* Available clients */}
              <div className="space-y-2 max-h-32 overflow-y-auto border rounded p-2">
                {activeClientKeys.length === 0 ? (
                  <div className="text-xs text-muted-foreground text-center py-2">
                    <p>{t("secrets.dialog.multiClient.noActiveClients")}</p>
                    <Link
                      to="/clients"
                      className="inline-flex items-center gap-1 text-foreground underline mt-1"
                    >
                      <ExternalLink className="h-3 w-3" /> Open Clients
                    </Link>
                  </div>
                ) : (
                  activeClientKeys.map((client) => (
                    <div
                      key={client.id}
                      className="flex items-center space-x-2"
                    >
                      <Checkbox
                        id={client.id}
                        checked={selectedClients.includes(client.id)}
                        onCheckedChange={(checked) =>
                          handleClientToggle(client.id, checked === true)
                        }
                      />
                      <Label
                        htmlFor={client.id}
                        className="text-xs font-mono cursor-pointer flex-1"
                      >
                        {client.name || `${client.id.substring(0, 12)}...`}
                        <span className="text-muted-foreground ml-1">
                          {client.description || client.id.substring(0, 8)}
                        </span>
                      </Label>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </ScrollArea>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={handleCancel}
            size="sm"
            disabled={createSecretMutation.isPending}
          >
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleAddSecret}
            size="sm"
            disabled={createSecretMutation.isPending}
          >
            {createSecretMutation.isPending && (
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
            )}
            {t("secrets.controls.addSecret")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
