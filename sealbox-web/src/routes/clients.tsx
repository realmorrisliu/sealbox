import { createFileRoute } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { useState } from "react";
import { format } from "date-fns";
import { enUS, zhCN, ja, de } from "date-fns/locale";
import {
  Key,
  Plus,
  RotateCw,
  AlertTriangle,
  Shield,
  Clock,
  MoreHorizontal,
} from "lucide-react";
import { toast } from "sonner";

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
import { Alert } from "@/components/ui/alert";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PageContainer } from "@/components/layout/page-container";
import { PageLayout } from "@/components/layout/page-layout";
import { DataSection } from "@/components/common/data-section";
import { EmptyState } from "@/components/common/empty-state";
import { ContentCard } from "@/components/common/content-card";
import { ClientKeyListSkeleton } from "@/components/common/loading-skeletons";
import {
  useClientKeys,
  useCreateClientKey,
  useRotateClientKey,
  useApproveEnrollment,
  useRenameClient,
  useUpdateClientStatus,
} from "@/hooks/use-api";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { AddClientDialog } from "@/components/clients/add-client-dialog";
import { StatusDot } from "@/components/common/status-dot";
import { formatDistanceToNowStrict } from "date-fns";
import type { ClientKey } from "@/lib/types";

export const Route = createFileRoute("/clients")({
  component: KeysPage,
});

function KeysPage() {
  return (
    <PageContainer>
      <ClientKeysPage />
    </PageContainer>
  );
}

function ClientKeysPage() {
  const { data: clientKeysData, isLoading, error, refetch } = useClientKeys();
  const createClientKey = useCreateClientKey();
  const rotateClientKey = useRotateClientKey();
  const approveEnrollment = useApproveEnrollment();
  const renameClient = useRenameClient();
  const updateClientStatus = useUpdateClientStatus();
  const { t, i18n } = useTranslation();
  const [query, setQuery] = useState("");

  const [enrollCode, setEnrollCode] = useState("");
  const [enrollName, setEnrollName] = useState("");
  const [enrollDesc, setEnrollDesc] = useState("");
  const [renameOpen, setRenameOpen] = useState(false);
  const [renameName, setRenameName] = useState("");
  const [renameDesc, setRenameDesc] = useState("");
  const [selectedClient, setSelectedClient] = useState<ClientKey | null>(null);
  const [addOpen, setAddOpen] = useState(false);

  const formatTimestamp = (timestamp: number) => {
    const getLocale = () => {
      switch (i18n.language) {
        case "zh":
          return zhCN;
        case "ja":
          return ja;
        case "de":
          return de;
        default:
          return enUS;
      }
    };

    return format(new Date(timestamp * 1000), "yyyy-MM-dd HH:mm:ss", {
      locale: getLocale(),
    });
  };

  const getStatusBadge = (status: ClientKey["status"]) => {
    switch (status) {
      case "Active":
        return (
          <Badge
            variant="default"
            className="inline-flex items-center space-x-1"
          >
            <Shield className="h-3 w-3" />
            <span className="text-xs">{t("keys.active")}</span>
          </Badge>
        );
      case "Retired":
        return (
          <Badge
            variant="secondary"
            className="inline-flex items-center space-x-1"
          >
            <Clock className="h-3 w-3" />
            <span className="text-xs">{t("keys.retired")}</span>
          </Badge>
        );
      case "Disabled":
        return (
          <Badge
            variant="destructive"
            className="inline-flex items-center space-x-1"
          >
            <AlertTriangle className="h-3 w-3" />
            <span className="text-xs">{t("keys.disabled")}</span>
          </Badge>
        );
      default:
        return (
          <Badge variant="outline" className="text-xs">
            {status}
          </Badge>
        );
    }
  };

  const handleRegisterKey = () => {
    toast.info(t("keys.cliRequired.title"), {
      description: t("keys.cliRequired.registerDescription"),
      duration: 8000,
    });
  };

  const handleRotateKey = () => {
    toast.info(t("keys.cliRequired.title"), {
      description: t("keys.cliRequired.rotateDescription"),
      duration: 8000,
    });
  };

  const clientKeys = clientKeysData?.client_keys || [];
  const rows = clientKeys.filter((c: any) => {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return (
      (c.name || "").toLowerCase().includes(q) ||
      (c.description || "").toLowerCase().includes(q) ||
      (c.id || "").toLowerCase().includes(q)
    );
  });

  return (
    <PageLayout
      title="Clients"
      subtitle="Manage devices connected to your sealbox server."
      stats={{
        count: rows.length,
        label: "clients",
        filtered: !!query,
      }}
      searchProps={{
        value: query,
        onChange: setQuery,
        placeholder: "Search clients...",
        size: "md",
      }}
      actions={
        <Button onClick={() => setAddOpen(true)}>
          <Plus className="h-4 w-4 mr-2" /> Add client
        </Button>
      }
    >
      <DataSection
        loading={isLoading}
        error={error}
        empty={clientKeys.length === 0}
        loadingSkeleton={<ClientKeyListSkeleton />}
        emptyState={
          <EmptyState
            icon={<Key className="h-5 w-5" />}
            title="Get your first client online"
            description="On the device, run the CLI to begin enrollment and get a code. Then approve the code here."
            centered={false}
            withContainer={true}
          >
            <div className="grid grid-cols-2 gap-6 items-start">
              <div className="space-y-2">
                <code className="bg-muted px-2 py-1 rounded text-xs font-mono">
                  sealbox-cli up --enroll
                </code>
                <p className="text-xs text-muted-foreground">
                  The CLI prints a code like ABCD-EFGH and a verify URL.
                </p>
              </div>
              <div className="space-y-3">
                <div>
                  <Label htmlFor="enrollCode" className="text-xs">
                    Enrollment Code
                  </Label>
                  <Input
                    id="enrollCode"
                    value={enrollCode}
                    onChange={(e) =>
                      setEnrollCode(e.target.value.toUpperCase())
                    }
                    placeholder="ABCD-EFGH"
                    className="h-8"
                  />
                </div>
                <div>
                  <Label htmlFor="enrollName" className="text-xs">
                    Name (optional)
                  </Label>
                  <Input
                    id="enrollName"
                    value={enrollName}
                    onChange={(e) => setEnrollName(e.target.value)}
                    placeholder="e.g., dev-laptop"
                    className="h-8"
                  />
                </div>
                <div>
                  <Label htmlFor="enrollDesc" className="text-xs">
                    Description (optional)
                  </Label>
                  <Input
                    id="enrollDesc"
                    value={enrollDesc}
                    onChange={(e) => setEnrollDesc(e.target.value)}
                    placeholder="Owner or purpose"
                    className="h-8"
                  />
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    onClick={async () => {
                      if (!enrollCode.trim()) return;
                      try {
                        await approveEnrollment.mutateAsync({
                          code: enrollCode.trim(),
                          name: enrollName || undefined,
                          description: enrollDesc || undefined,
                        });
                        toast.success(t("common.success") || "Approved", {
                          description:
                            "Enrollment approved. The client will appear once CLI completes.",
                        });
                      } catch (e: any) {
                        toast.error(t("common.error") || "Error", {
                          description: e?.message || "Approval failed",
                        });
                      }
                    }}
                    disabled={approveEnrollment.isPending || !enrollCode.trim()}
                    className="border-border"
                  >
                    {approveEnrollment.isPending && (
                      <RotateCw className="h-4 w-4 mr-2 animate-spin" />
                    )}
                    Approve Enrollment
                  </Button>
                  <Button
                    variant="ghost"
                    onClick={() => {
                      setEnrollCode("");
                      setEnrollName("");
                      setEnrollDesc("");
                    }}
                  >
                    Clear
                  </Button>
                </div>
              </div>
            </div>
          </EmptyState>
        }
        errorProps={{
          title: t("common.loadingFailed"),
          description: error?.message,
          onRetry: () => refetch(),
          retryLabel: t("common.retry"),
        }}
      >
        <ContentCard>
          <div className="p-1 overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow className="border-b border-border hover:bg-transparent">
                  <TableHead className="font-medium text-foreground">
                    Client
                  </TableHead>
                  <TableHead className="font-medium text-foreground">
                    Status
                  </TableHead>
                  <TableHead className="font-medium text-foreground table-cell">
                    Created
                  </TableHead>
                  <TableHead className="font-medium text-foreground table-cell">
                    Last seen
                  </TableHead>
                  <TableHead className="font-medium text-foreground text-right">
                    Actions
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map((key: ClientKey) => (
                  <TableRow
                    key={key.id}
                    className="min-h-12 border-b border-border hover:bg-accent/50 transition-colors duration-150"
                  >
                    <TableCell className="font-medium">
                      <div className="min-w-0">
                        <div
                          className="text-sm font-medium truncate"
                          title={key.name || key.id}
                        >
                          {key.name || `${key.id.substring(0, 16)}...`}
                        </div>
                        <div className="text-xs text-muted-foreground font-mono truncate">
                          {key.id.substring(0, 16)}...
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>
                      {key.status === "Active" ? (
                        <div className="flex items-center gap-2">
                          <StatusDot tone="success" title="Active" />
                          <span className="text-xs text-success">Active</span>
                        </div>
                      ) : key.status === "Retired" ? (
                        <div className="flex items-center gap-2">
                          <StatusDot tone="warning" title="Retired" />
                          <span className="text-xs text-warning">Retired</span>
                        </div>
                      ) : (
                        <div className="flex items-center gap-2">
                          <StatusDot tone="destructive" title="Disabled" />
                          <span className="text-xs text-destructive">
                            Disabled
                          </span>
                        </div>
                      )}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground table-cell">
                      {formatTimestamp(key.created_at)}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground table-cell">
                      {key.last_used_at ? (
                        <span className="whitespace-nowrap">
                          {formatDistanceToNowStrict(
                            new Date(key.last_used_at * 1000),
                            { addSuffix: true },
                          )}
                        </span>
                      ) : (
                        <span className="text-muted-foreground/70">—</span>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      <RowActions
                        client={key}
                        onRename={() => {
                          setSelectedClient(key);
                          setRenameName(key.name || "");
                          setRenameDesc(key.description || "");
                          setRenameOpen(true);
                        }}
                        onSetStatus={(status) => {
                          updateClientStatus.mutate({
                            clientId: key.id,
                            status,
                          });
                        }}
                      />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </ContentCard>
      </DataSection>

      <AddClientDialog open={addOpen} onOpenChange={setAddOpen} />

      {/* Rename dialog */}
      <Dialog open={renameOpen} onOpenChange={setRenameOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Client</DialogTitle>
          </DialogHeader>
          <div className="space-y-3">
            <div>
              <Label htmlFor="clientName" className="text-xs">
                Name
              </Label>
              <Input
                id="clientName"
                value={renameName}
                onChange={(e) => setRenameName(e.target.value)}
                className="h-8"
              />
            </div>
            <div>
              <Label htmlFor="clientDesc" className="text-xs">
                Description
              </Label>
              <Input
                id="clientDesc"
                value={renameDesc}
                onChange={(e) => setRenameDesc(e.target.value)}
                className="h-8"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRenameOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={async () => {
                if (!selectedClient) return;
                try {
                  await renameClient.mutateAsync({
                    clientId: selectedClient.id,
                    name: renameName,
                    description: renameDesc || undefined,
                  });
                  toast.success("Client updated");
                  setRenameOpen(false);
                } catch (e: any) {
                  toast.error("Update failed", {
                    description: e?.message || "",
                  });
                }
              }}
              disabled={renameClient.isPending}
            >
              {renameClient.isPending && (
                <RotateCw className="h-4 w-4 mr-2 animate-spin" />
              )}
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PageLayout>
  );
}

function RowActions({
  client,
  onRename,
  onSetStatus,
}: {
  client: ClientKey;
  onRename: () => void;
  onSetStatus: (s: "Active" | "Disabled" | "Retired") => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
          <MoreHorizontal className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-44">
        <DropdownMenuLabel>Client</DropdownMenuLabel>
        <DropdownMenuItem onClick={onRename}>Rename</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>Status</DropdownMenuLabel>
        <DropdownMenuItem onClick={() => onSetStatus("Active")}>
          Active
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => onSetStatus("Disabled")}>
          Disabled
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => onSetStatus("Retired")}>
          Retired
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
