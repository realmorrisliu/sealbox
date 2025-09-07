"use client";

import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { useApproveEnrollment } from "@/hooks/use-api";

export function AddClientDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const approve = useApproveEnrollment();
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add client</DialogTitle>
        </DialogHeader>
        <div className="space-y-3">
          <div className="text-xs text-muted-foreground">
            On the device, run:{" "}
            <code className="bg-muted px-1 py-0.5 rounded">
              sealbox-cli up --enroll
            </code>{" "}
            and paste the code here.
          </div>
          <div>
            <Label htmlFor="enroll-code" className="text-xs">
              Enrollment code
            </Label>
            <Input
              id="enroll-code"
              value={code}
              onChange={(e) => setCode(e.target.value.toUpperCase())}
              placeholder="ABCD-EFGH"
              className="h-8"
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <Label htmlFor="client-name" className="text-xs">
                Name
              </Label>
              <Input
                id="client-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g., laptop"
                className="h-8"
              />
            </div>
            <div>
              <Label htmlFor="client-desc" className="text-xs">
                Description
              </Label>
              <Input
                id="client-desc"
                value={desc}
                onChange={(e) => setDesc(e.target.value)}
                placeholder="Owner or purpose"
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
            Cancel
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
            Approve & Add
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
