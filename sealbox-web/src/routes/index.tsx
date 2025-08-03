// src/routes/index.tsx
import { createFileRoute } from "@tanstack/react-router";
import { format } from "date-fns";
import { enUS } from "date-fns/locale";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Alert } from "@/components/ui/alert";
import { AuthGuard } from "@/components/auth/auth-guard";
import { MainLayout } from "@/components/layout/main-layout";
import { useSecrets, useDeleteSecret } from "@/hooks/use-api";
import { Plus, Trash2, Eye, Clock, AlertTriangle } from "lucide-react";
import type { SecretInfo } from "@/lib/types";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <AuthGuard>
      <MainLayout>
        <SecretsPage />
      </MainLayout>
    </AuthGuard>
  );
}

function SecretsPage() {
  const { data: secretsData, isLoading, error, refetch } = useSecrets();
  const deleteSecret = useDeleteSecret();

  const handleDeleteSecret = async (key: string, version: number) => {
    if (confirm(`Are you sure you want to delete secret "${key}" version ${version}?`)) {
      try {
        await deleteSecret.mutateAsync({ key, version });
      } catch (error) {
        console.error("Delete failed:", error);
        alert("Delete failed, please try again");
      }
    }
  };

  const formatTimestamp = (timestamp: number) => {
    return format(new Date(timestamp * 1000), "yyyy-MM-dd HH:mm:ss", {
      locale: enUS,
    });
  };

  const getExpiryStatus = (expiresAt?: number) => {
    if (!expiresAt) return null;
    
    const now = Date.now() / 1000;
    const timeUntilExpiry = expiresAt - now;
    
    if (timeUntilExpiry <= 0) {
      return { status: "expired", text: "Expired", color: "text-red-500" };
    }
    
    if (timeUntilExpiry < 3600) { // Within 1 hour
      return { 
        status: "warning", 
        text: `Expires in ${Math.ceil(timeUntilExpiry / 60)}m`, 
        color: "text-orange-500" 
      };
    }
    
    if (timeUntilExpiry < 86400) { // Within 24 hours
      return { 
        status: "warning", 
        text: `Expires in ${Math.ceil(timeUntilExpiry / 3600)}h`, 
        color: "text-orange-500" 
      };
    }
    
    const days = Math.ceil(timeUntilExpiry / 86400);
    return { 
      status: "normal", 
      text: `Expires in ${days}d`, 
      color: "text-muted-foreground" 
    };
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <p>Loading...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-4">
        <Alert variant="destructive">
          <AlertTriangle className="h-4 w-4" />
          <div>
            <p className="font-medium">Loading Failed</p>
            <p className="text-sm">{error.message}</p>
          </div>
        </Alert>
        <Button onClick={() => refetch()}>Retry</Button>
      </div>
    );
  }

  const secrets = secretsData?.secrets || [];

  return (
    <div className="space-y-6">
      {/* Page title and actions */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Secret Management</h1>
          <p className="text-muted-foreground">
            Manage your encrypted secrets and sensitive data
          </p>
        </div>
        <Button disabled>
          <Plus className="h-4 w-4 mr-2" />
          New Secret (Coming Soon)
        </Button>
      </div>

      {/* Secrets list */}
      <Card>
        {secrets.length === 0 ? (
          <div className="p-8 text-center">
            <p className="text-muted-foreground mb-4">No secrets found</p>
            <Button disabled>
              <Plus className="h-4 w-4 mr-2" />
              Create Your First Secret (Coming Soon)
            </Button>
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Secret Name</TableHead>
                <TableHead>Version</TableHead>
                <TableHead>Created At</TableHead>
                <TableHead>Updated At</TableHead>
                <TableHead>Expires At</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {secrets.map((secret: SecretInfo) => {
                const expiryStatus = getExpiryStatus(secret.expires_at);
                
                return (
                  <TableRow key={`${secret.key}-${secret.version}`}>
                    <TableCell className="font-medium">
                      {secret.key}
                    </TableCell>
                    <TableCell>v{secret.version}</TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatTimestamp(secret.created_at)}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatTimestamp(secret.updated_at)}
                    </TableCell>
                    <TableCell>
                      {secret.expires_at ? (
                        <div className="flex items-center space-x-1">
                          <Clock className="h-3 w-3" />
                          <span className={`text-xs ${expiryStatus?.color}`}>
                            {expiryStatus?.text}
                          </span>
                        </div>
                      ) : (
                        <span className="text-xs text-muted-foreground">Never expires</span>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end space-x-2">
                        <Button variant="ghost" size="sm" disabled>
                          <Eye className="h-3 w-3" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleDeleteSecret(secret.key, secret.version)}
                          disabled={deleteSecret.isPending}
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        )}
      </Card>
    </div>
  );
}
