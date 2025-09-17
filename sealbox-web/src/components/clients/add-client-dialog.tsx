"use client";

import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { useApproveEnrollment } from "@/hooks/use-api";
import { Loader2 } from "lucide-react";

export function AddClientDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const { t } = useTranslation();
  const approve = useApproveEnrollment();
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="text-lg">{t("components.addClient.title")}</DialogTitle>
          <DialogDescription className="text-sm">
            On the device, run
            {" "}
            <code className="bg-muted px-1 py-0.5 rounded">sealbox-cli up --enroll</code>
            {" "}
            and paste the enrollment code below.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div>
            <Label htmlFor="enroll-code" className="text-xs">
              {t("components.addClient.enrollmentCode")}
            </Label>
            <Input
              id="enroll-code"
              value={code}
              onChange={(e) => setCode(e.target.value.toUpperCase())}
              placeholder={t("components.addClient.enrollmentCodePlaceholder")}
              className="h-8"
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <Label htmlFor="client-name" className="text-xs">
                {t("components.addClient.name")}
              </Label>
              <Input
                id="client-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t("components.addClient.namePlaceholder")}
                className="h-8"
              />
            </div>
            <div>
              <Label htmlFor="client-desc" className="text-xs">
                {t("components.addClient.description")}
              </Label>
              <Input
                id="client-desc"
                value={desc}
                onChange={(e) => setDesc(e.target.value)}
                placeholder={t("components.addClient.descriptionPlaceholder")}
                className="h-8"
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            size="sm"
          >
            {t("components.addClient.cancel")}
          </Button>
          <Button
            size="sm"
            onClick={async () => {
              if (!code.trim()) return;
              await approve.mutateAsync({
                code: code.trim(),
                name: name || undefined,
                description: desc || undefined,
              });
              onOpenChange(false);
            }}
            disabled={approve.isPending || !code.trim()}
          >
            {approve.isPending && (
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
            )}
            {t("components.addClient.approveAdd")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
