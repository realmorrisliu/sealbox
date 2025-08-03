// src/routes/index.tsx
import { createFileRoute } from "@tanstack/react-router";
import { format } from "date-fns";
import { enUS, zhCN, ja, de } from "date-fns/locale";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
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
  const { t, i18n } = useTranslation();

  const handleDeleteSecret = async (key: string, version: number) => {
    if (confirm(t('secrets.deleteConfirm', { key, version }))) {
      try {
        await deleteSecret.mutateAsync({ key, version });
      } catch (error) {
        console.error("Delete failed:", error);
        alert(t('secrets.deleteFailed'));
      }
    }
  };

  const formatTimestamp = (timestamp: number) => {
    const getLocale = () => {
      switch (i18n.language) {
        case 'zh': return zhCN;
        case 'ja': return ja;
        case 'de': return de;
        default: return enUS;
      }
    };
    
    return format(new Date(timestamp * 1000), "yyyy-MM-dd HH:mm:ss", {
      locale: getLocale(),
    });
  };

  const getExpiryStatus = (expiresAt?: number) => {
    if (!expiresAt) return null;
    
    const now = Date.now() / 1000;
    const timeUntilExpiry = expiresAt - now;
    
    if (timeUntilExpiry <= 0) {
      return { status: "expired", text: t('secrets.expired'), color: "text-red-500" };
    }
    
    if (timeUntilExpiry < 3600) { // Within 1 hour
      return { 
        status: "warning", 
        text: t('secrets.expiresInMinutes', { minutes: Math.ceil(timeUntilExpiry / 60) }), 
        color: "text-orange-500" 
      };
    }
    
    if (timeUntilExpiry < 86400) { // Within 24 hours
      return { 
        status: "warning", 
        text: t('secrets.expiresInHours', { hours: Math.ceil(timeUntilExpiry / 3600) }), 
        color: "text-orange-500" 
      };
    }
    
    const days = Math.ceil(timeUntilExpiry / 86400);
    return { 
      status: "normal", 
      text: t('secrets.expiresInDays', { days }), 
      color: "text-muted-foreground" 
    };
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <p>{t('common.loading')}</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-4">
        <Alert variant="destructive">
          <AlertTriangle className="h-4 w-4" />
          <div>
            <p className="font-medium">{t('common.loadingFailed')}</p>
            <p className="text-sm">{error.message}</p>
          </div>
        </Alert>
        <Button onClick={() => refetch()}>{t('common.retry')}</Button>
      </div>
    );
  }

  const secrets = secretsData?.secrets || [];

  return (
    <div className="space-section">
      {/* [顶层] 页面标题 + 主要操作 - 按指南Step 4 */}
      <div className="flex items-start justify-between">
        <div className="space-tight">
          <h1 className="text-4xl font-bold tracking-tight">{t('secrets.title')}</h1>
          <p className="text-sm text-muted-foreground">
            {t('secrets.description')}
          </p>
        </div>
        <Button disabled variant="outline" className="border-border">
          <Plus className="h-4 w-4 mr-2" />
          {t('secrets.newSecretComingSoon')}
        </Button>
      </div>

      {/* [中层] 内容分区 - Section卡片 */}
      <Card className="bg-card border border-border">
        {secrets.length === 0 ? (
          <div className="padding-section text-center space-content">
            <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-4">
              <Plus className="h-6 w-6 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground text-sm">{t('secrets.noSecretsFound')}</p>
            <Button disabled variant="outline">
              <Plus className="h-4 w-4 mr-2" />
              {t('secrets.createFirst')}
            </Button>
          </div>
        ) : (
          <>
            {/* Mobile Card View */}
            <div className="block sm:hidden space-y-4 p-6">
              {secrets.map((secret: SecretInfo) => {
                const expiryStatus = getExpiryStatus(secret.expires_at);
                
                return (
                  <div 
                    key={`${secret.key}-${secret.version}`}
                    className="bg-card border border-border rounded-md p-4 space-y-4 hover:bg-accent/50 transition-colors duration-150"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-mono text-sm font-medium">{secret.key}</span>
                      <Badge variant="secondary" className="font-mono text-xs">
                        v{secret.version}
                      </Badge>
                    </div>
                    
                    <div className="space-y-2">
                      <div className="flex justify-between text-xs text-muted-foreground">
                        <span>{t('secrets.createdAt')}</span>
                        <span>{formatTimestamp(secret.created_at)}</span>
                      </div>
                      
                      <div className="flex justify-between items-center">
                        <span className="text-xs text-muted-foreground">{t('secrets.expiresAt')}</span>
                        {secret.expires_at ? (
                          <Badge 
                            variant={
                              expiryStatus?.status === "expired" ? "destructive" :
                              expiryStatus?.status === "warning" ? "default" : "secondary"
                            }
                            className="flex items-center space-x-1"
                          >
                            <Clock className="h-3 w-3" />
                            <span className="text-xs">
                              {expiryStatus?.text}
                            </span>
                          </Badge>
                        ) : (
                          <Badge variant="outline" className="text-xs">
                            {t('secrets.neverExpires')}
                          </Badge>
                        )}
                      </div>
                    </div>
                    
                    <div className="flex items-center justify-end space-x-2 pt-2 border-t border-border">
                      <Button 
                        variant="ghost" 
                        size="sm" 
                        disabled
                        className="h-8 px-3 hover:bg-accent transition-colors duration-150"
                      >
                        <Eye className="h-3 w-3 mr-1" />
                        <span className="text-xs">{t('common.view')}</span>
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDeleteSecret(secret.key, secret.version)}
                        disabled={deleteSecret.isPending}
                        className="h-8 px-3 hover:bg-destructive/10 hover:text-destructive transition-colors duration-150"
                      >
                        <Trash2 className="h-3 w-3 mr-1" />
                        <span className="text-xs">{t('common.delete')}</span>
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Desktop Table View - Linear风格表格 */}
            <div className="hidden sm:block overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow className="border-b border-border hover:bg-transparent">
                    <TableHead className="font-medium text-foreground">{t('secrets.secretName')}</TableHead>
                    <TableHead className="font-medium text-foreground">{t('secrets.version')}</TableHead>
                    <TableHead className="font-medium text-foreground hidden md:table-cell">{t('secrets.createdAt')}</TableHead>
                    <TableHead className="font-medium text-foreground hidden lg:table-cell">{t('secrets.updatedAt')}</TableHead>
                    <TableHead className="font-medium text-foreground">{t('secrets.expiresAt')}</TableHead>
                    <TableHead className="font-medium text-foreground text-right">{t('secrets.actions')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {secrets.map((secret: SecretInfo) => {
                    const expiryStatus = getExpiryStatus(secret.expires_at);
                    
                    return (
                      <TableRow 
                        key={`${secret.key}-${secret.version}`}
                        className="h-12 border-b border-border hover:bg-accent/50 transition-colors duration-150 cursor-pointer"
                      >
                        <TableCell className="font-medium">
                          <span className="font-mono text-sm">{secret.key}</span>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary" className="font-mono text-xs">
                            v{secret.version}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground hidden md:table-cell">
                          {formatTimestamp(secret.created_at)}
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground hidden lg:table-cell">
                          {formatTimestamp(secret.updated_at)}
                        </TableCell>
                        <TableCell>
                          {secret.expires_at ? (
                            <Badge 
                              variant={
                                expiryStatus?.status === "expired" ? "destructive" :
                                expiryStatus?.status === "warning" ? "default" : "secondary"
                              }
                              className="flex items-center space-x-1 w-fit"
                            >
                              <Clock className="h-3 w-3" />
                              <span className="text-xs">
                                {expiryStatus?.text}
                              </span>
                            </Badge>
                          ) : (
                            <Badge variant="outline" className="text-xs">
                              {t('secrets.neverExpires')}
                            </Badge>
                          )}
                        </TableCell>
                        <TableCell className="text-right">
                          <div className="flex items-center justify-end space-x-2">
                            <Button 
                              variant="ghost" 
                              size="sm" 
                              disabled
                              className="h-8 w-8 p-0 hover:bg-accent transition-colors duration-150"
                            >
                              <Eye className="h-3 w-3" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => handleDeleteSecret(secret.key, secret.version)}
                              disabled={deleteSecret.isPending}
                              className="h-8 w-8 p-0 hover:bg-destructive/10 hover:text-destructive transition-colors duration-150"
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
            </div>
          </>
        )}
      </Card>
    </div>
  );
}
