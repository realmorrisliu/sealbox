"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { Plus, Loader2, X, ExternalLink, Trash2 } from "lucide-react";
import { Link } from "@tanstack/react-router";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useCreateSecret, useClientKeys } from "@/hooks/use-api";
import type { CreateSecretRequest } from "@/lib/types";
import { ScrollArea } from "@/components/ui/scroll-area";

interface CreateSecretDialogProps {
  children?: React.ReactNode;
}

export function CreateSecretDialog({ children }: CreateSecretDialogProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  // Bulk-add state
  const [pasteText, setPasteText] = useState("");
  const [rows, setRows] = useState<
    Array<{ name: string; value: string; ttl: string }>
  >([{ name: "", value: "", ttl: "" }]);
  const [appendMode, setAppendMode] = useState(false);
  const [selectedClients, setSelectedClients] = useState<string[]>([]);
  const createSecretMutation = useCreateSecret();
  const { data: clientKeysData } = useClientKeys();

  const [isSaving, setIsSaving] = useState(false);
  const validRowsCount = useMemo(
    () => rows.filter((r) => r.name.trim() && r.value.trim()).length,
    [rows],
  );

  const parsedFromPaste = useMemo(() => {
    const lines = pasteText
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean);
    const next: Array<{ name: string; value: string; ttl: string }> = [];
    for (const line of lines) {
      // Support "name,value[,ttl]" or "name=value[,ttl]"
      const hasEquals = line.includes("=");
      const parts = hasEquals ? line.split("=") : line.split(",");
      if (parts.length < 2) continue;
      const name = parts[0]!.trim();
      const rest = parts.slice(1).join(hasEquals ? "=" : ",");
      // rest may contain comma + ttl, split once from end if present
      const lastComma = rest.lastIndexOf(",");
      let value = rest;
      let ttl = "";
      if (lastComma !== -1) {
        value = rest.slice(0, lastComma);
        ttl = rest.slice(lastComma + 1);
      }
      const clean = (s: string) => s.trim().replace(/^"|"$/g, "");
      const row = { name: clean(name), value: clean(value), ttl: clean(ttl) };
      if (row.name && row.value) next.push(row);
    }
    return next;
  }, [pasteText]);

  // We apply parsing on paste (native event) or via the Apply button.
  const applyParsed = (text: string) => {
    const parsed = (() => {
      const lines = text
        .split(/\r?\n/)
        .map((l) => l.trim())
        .filter(Boolean);
      const next: Array<{ name: string; value: string; ttl: string }> = [];
      for (const line of lines) {
        const hasEquals = line.includes("=");
        const parts = hasEquals ? line.split("=") : line.split(",");
        if (parts.length < 2) continue;
        const name = parts[0]!.trim();
        const rest = parts.slice(1).join(hasEquals ? "=" : ",");
        const lastComma = rest.lastIndexOf(",");
        let value = rest;
        let ttl = "";
        if (lastComma !== -1) {
          value = rest.slice(0, lastComma);
          ttl = rest.slice(lastComma + 1);
        }
        const clean = (s: string) => s.trim().replace(/^"|"$/g, "");
        const row = { name: clean(name), value: clean(value), ttl: clean(ttl) };
        if (row.name && row.value) next.push(row);
      }
      return next;
    })();

    if (parsed.length === 0) return;
    setRows((prev) => (appendMode ? [...prev, ...parsed] : parsed));
  };

  const handleAddSecret = async () => {
    const validRows = rows.filter((r) => r.name && r.value);
    if (validRows.length === 0) {
      toast.error(t("secrets.dialog.missingFields"), {
        description: t("secrets.dialog.nameAndValueRequired"),
      });
      return;
    }
    if (selectedClients.length === 0) {
      toast.error(t("secrets.dialog.multiClient.noClientsSelected"), {
        description: t("secrets.dialog.multiClient.selectAtLeastOne"),
      });
      return;
    }

    setIsSaving(true);
    let success = 0;
    let failed = 0;
    for (const row of validRows) {
      const data: CreateSecretRequest = {
        secret: row.value,
        ...(row.ttl && { ttl: parseInt(row.ttl, 10) }),
        authorized_clients: selectedClients,
      };
      try {
        await createSecretMutation.mutateAsync({ key: row.name, data });
        success += 1;
      } catch (e) {
        failed += 1;
      }
    }
    setIsSaving(false);

    if (success > 0 && failed === 0) {
      toast.success(t("secrets.dialog.secretAdded"), {
        description: t("secrets.dialog.bulk.summarySuccess", {
          success,
          total: validRows.length,
        }),
      });
      setPasteText("");
      setRows([{ name: "", value: "", ttl: "" }]);
      setSelectedClients([]);
      setOpen(false);
    } else {
      toast[success > 0 ? "warning" : "error"](t("common.error"), {
        description: t("secrets.dialog.bulk.summaryPartial", {
          success,
          failed,
        }),
      });
    }
  };

  const handleCancel = () => {
    setPasteText("");
    setRows([{ name: "", value: "", ttl: "" }]);
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
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>{children}</SheetTrigger>
      <SheetContent side="right" className="sm:max-w-3xl">
        <div className="flex h-full min-h-0 flex-col">
          <SheetHeader className="border-b px-6 py-5">
            <SheetTitle className="text-lg">
              {t("secrets.dialog.addNewSecret")}
            </SheetTitle>
            <SheetDescription className="text-sm">
              {t("secrets.dialog.addNewSecretDescription")} Bulk add is
              supported.
            </SheetDescription>
          </SheetHeader>
          <ScrollArea className="flex-1 min-h-0">
            <div className="p-6 space-y-6">
              {/* Paste area */}
              <div>
                <Label htmlFor="paste" className="text-xs mb-1.5">
                  {t("secrets.dialog.bulk.pasteLabel")}
                </Label>
                <Textarea
                  id="paste"
                  placeholder={t("secrets.dialog.bulk.pastePlaceholder")}
                  value={pasteText}
                  onChange={(e) => setPasteText(e.target.value)}
                  onPaste={(e) => {
                    const text = e.clipboardData?.getData("text") || "";
                    if (text) {
                      e.preventDefault();
                      setPasteText(text);
                      applyParsed(text);
                    }
                  }}
                  className="min-h-24 font-mono resize-none"
                />
                <div className="flex items-center justify-between mt-2">
                  <div className="text-xs text-muted-foreground">
                    {t("secrets.dialog.bulk.pasteHelp")}
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="flex items-center gap-1 text-xs">
                      <Checkbox
                        id="append-mode"
                        checked={appendMode}
                        onCheckedChange={(v) => setAppendMode(v === true)}
                      />
                      <Label
                        htmlFor="append-mode"
                        className="text-xs cursor-pointer"
                      >
                        {t("secrets.dialog.bulk.append")}
                      </Label>
                    </div>
                  </div>
                </div>
                <div className="flex justify-start mt-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => applyParsed(pasteText)}
                  >
                    {t("secrets.dialog.bulk.apply")}
                  </Button>
                </div>
              </div>

              {/* Rows editor */}
              <div>
                <div className="mb-1.5">
                  <Label className="text-xs">
                    {t("secrets.dialog.bulk.rowsLabel")}
                  </Label>
                </div>
                <div className="grid grid-cols-12 gap-2 text-xs text-muted-foreground mb-2 px-1">
                  <div className="col-span-3">{t("secrets.dialog.bulk.columns.name")}</div>
                  <div className="col-span-6">{t("secrets.dialog.bulk.columns.value")}</div>
                  <div className="col-span-2">{t("secrets.dialog.bulk.columns.ttl")}</div>
                  <div className="col-span-1"></div>
                </div>
                <div className="space-y-2">
                  {rows.map((row, idx) => (
                    <div
                      key={idx}
                      className="grid grid-cols-12 gap-2 items-center"
                    >
                      <Input
                        className="col-span-3 h-8"
                        placeholder={t("secrets.dialog.nameHelp")}
                        value={row.name}
                        onChange={(e) =>
                          setRows((rs) =>
                            rs.map((r, i) =>
                              i === idx ? { ...r, name: e.target.value } : r,
                            ),
                          )
                        }
                      />
                      <Input
                        className="col-span-6 h-8"
                        placeholder={t("secrets.dialog.valueHelp")}
                        value={row.value}
                        onChange={(e) =>
                          setRows((rs) =>
                            rs.map((r, i) =>
                              i === idx ? { ...r, value: e.target.value } : r,
                            ),
                          )
                        }
                      />
                      <div className="col-span-2 relative">
                        <Input
                          className="h-8 pr-6"
                          inputMode="numeric"
                          type="number"
                          min={0}
                          step={1}
                          placeholder={t("secrets.dialog.ttlHelp")}
                          value={row.ttl}
                          onChange={(e) =>
                            setRows((rs) =>
                              rs.map((r, i) =>
                                i === idx ? { ...r, ttl: e.target.value } : r,
                              ),
                            )
                          }
                        />
                        <span className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                          {t("secrets.dialog.bulk.unitSeconds")}
                        </span>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="col-span-1 h-8 w-8"
                        onClick={() =>
                          setRows((rs) => rs.filter((_, i) => i !== idx))
                        }
                        disabled={rows.length === 1}
                        title={t("secrets.dialog.bulk.removeRow")}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  ))}
                </div>
                <div className="mt-3">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      setRows((r) => [...r, { name: "", value: "", ttl: "" }])
                    }
                  >
                    <Plus className="h-3.5 w-3.5 mr-1" />{" "}
                    {t("secrets.dialog.bulk.addRow")}
                  </Button>
                </div>
              </div>

              {/* Clients selection */}
              <div className="border-t pt-4">
                <Label className="text-xs mb-2">
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
                <div className="space-y-2 max-h-40 overflow-y-auto border rounded p-2">
                  {activeClientKeys.length === 0 ? (
                    <div className="text-xs text-muted-foreground text-center py-4">
                      <p>{t("secrets.dialog.multiClient.noActiveClients")}</p>
                      <Link
                        to="/clients"
                        className="inline-flex items-center gap-1 text-foreground underline mt-1"
                      >
                      <ExternalLink className="h-3 w-3" /> {t("secrets.dialog.bulk.openClients")}
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
          <SheetFooter className="border-t px-4 py-3 !justify-start">
            <Button
              onClick={handleAddSecret}
              size="sm"
              disabled={
                createSecretMutation.isPending ||
                isSaving ||
                validRowsCount === 0 ||
                selectedClients.length === 0
              }
            >
              {(createSecretMutation.isPending || isSaving) && (
                <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              )}
              {validRowsCount > 1
                ? t("secrets.dialog.bulk.createCount", { count: validRowsCount })
                : t("secrets.controls.addSecret")}
            </Button>
            <Button
              variant="outline"
              onClick={handleCancel}
              size="sm"
              disabled={createSecretMutation.isPending || isSaving}
            >
              {t("common.cancel")}
            </Button>
          </SheetFooter>
        </div>
      </SheetContent>
    </Sheet>
  );
}
