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
    <div className="space-section animate-fade-in">
      {/* Page title and actions */}
      <div className="flex flex-col sm:flex-row sm:items-start justify-between space-y-4 sm:space-y-0">
        <div className="space-tight">
          <h1 className="text-2xl sm:text-3xl lg:text-4xl font-bold tracking-tight text-balance">{t('secrets.title')}</h1>
          <p className="text-base sm:text-lg text-muted-foreground text-balance">
            {t('secrets.description')}
          </p>
        </div>
        <Button disabled variant="outline" className="bg-gradient-warm border-primary/20 w-full sm:w-auto gradient-transition">
          <Plus className="h-4 w-4 mr-2" />
          <span className="sm:hidden">{t('secrets.newSecret')}</span>
          <span className="hidden sm:inline">{t('secrets.newSecretComingSoon')}</span>
        </Button>
      </div>

      {/* Secrets list */}
      <Card className="bg-glass-enhanced overflow-hidden animate-scale-in card-hover-effect">
        {secrets.length === 0 ? (
          <div className="padding-section text-center space-content">
            <div className="w-16 h-16 bg-gradient-vibrant rounded-full flex items-center justify-center mx-auto mb-4 border border-primary/20 pulse-glow">
              <Plus className="h-6 w-6 text-primary" />
            </div>
            <p className="text-muted-foreground heading-sm">{t('secrets.noSecretsFound')}</p>
            <Button disabled variant="outline" className="bg-gradient-cool border-primary/20">
              <Plus className="h-4 w-4 mr-2" />
              {t('secrets.createFirst')}
            </Button>
          </div>
        ) : (
          <>
            {/* Mobile Card View */}
            <div className="block sm:hidden space-y-3 p-4">
              {secrets.map((secret: SecretInfo) => {
                const expiryStatus = getExpiryStatus(secret.expires_at);
                
                return (
                  <div 
                    key={`${secret.key}-${secret.version}`}
                    className="bg-textured-card border border-border/40 rounded-lg p-4 space-y-3 interactive-glow texture-hover"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center space-x-2">
                        <div className="w-2 h-2 bg-button-primary rounded-full shadow-sm" />
                        <span className="font-mono text-sm font-medium">{secret.key}</span>
                      </div>
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
                    
                    <div className="flex items-center justify-end space-x-2 pt-2 border-t border-border/20">
                      <Button 
                        variant="ghost" 
                        size="sm" 
                        disabled
                        className="h-8 px-3 hover:bg-accent/50 button-press"
                      >
                        <Eye className="h-3 w-3 mr-1" />
                        <span className="text-xs">{t('common.view')}</span>
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDeleteSecret(secret.key, secret.version)}
                        disabled={deleteSecret.isPending}
                        className="h-8 px-3 hover:bg-destructive/10 hover:text-destructive button-press"
                      >
                        <Trash2 className="h-3 w-3 mr-1" />
                        <span className="text-xs">{t('common.delete')}</span>
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Desktop Table View */}
            <div className="hidden sm:block overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow className="border-b border-primary/10 hover:bg-transparent bg-gradient-cool backdrop-blur-sm">
                    <TableHead className="font-semibold text-foreground/90">{t('secrets.secretName')}</TableHead>
                    <TableHead className="font-semibold text-foreground/90">{t('secrets.version')}</TableHead>
                    <TableHead className="font-semibold text-foreground/90 hidden md:table-cell">{t('secrets.createdAt')}</TableHead>
                    <TableHead className="font-semibold text-foreground/90 hidden lg:table-cell">{t('secrets.updatedAt')}</TableHead>
                    <TableHead className="font-semibold text-foreground/90">{t('secrets.expiresAt')}</TableHead>
                    <TableHead className="font-semibold text-foreground/90 text-right">{t('secrets.actions')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {secrets.map((secret: SecretInfo) => {
                    const expiryStatus = getExpiryStatus(secret.expires_at);
                    
                    return (
                      <TableRow 
                        key={`${secret.key}-${secret.version}`}
                        className="border-b border-border/20 hover:bg-gradient-warm interactive-element backdrop-blur-sm"
                      >
                        <TableCell className="font-medium py-4">
                          <div className="flex items-center space-x-2">
                            <div className="w-2 h-2 bg-button-primary rounded-full shadow-sm" />
                            <span className="font-mono text-sm">{secret.key}</span>
                          </div>
                        </TableCell>
                        <TableCell className="py-4">
                          <Badge variant="secondary" className="font-mono text-xs">
                            v{secret.version}
                          </Badge>
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground py-4 hidden md:table-cell">
                          {formatTimestamp(secret.created_at)}
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground py-4 hidden lg:table-cell">
                          {formatTimestamp(secret.updated_at)}
                        </TableCell>
                        <TableCell className="py-4">
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
                        <TableCell className="text-right py-4">
                          <div className="flex items-center justify-end space-x-1">
                            <Button 
                              variant="ghost" 
                              size="sm" 
                              disabled
                              className="h-8 w-8 p-0 hover:bg-accent/50 button-press"
                            >
                              <Eye className="h-3 w-3" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => handleDeleteSecret(secret.key, secret.version)}
                              disabled={deleteSecret.isPending}
                              className="h-8 w-8 p-0 hover:bg-destructive/10 hover:text-destructive button-press"
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
