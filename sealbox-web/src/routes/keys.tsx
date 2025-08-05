import { createFileRoute } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { useState } from "react";
import { format } from "date-fns";
import { enUS, zhCN, ja, de } from "date-fns/locale";
import { Key, Plus, RotateCw, AlertTriangle, Shield, Clock } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Alert } from "@/components/ui/alert";
import { AuthGuard } from "@/components/auth/auth-guard";
import { MainLayout } from "@/components/layout/main-layout";
import { MasterKeyListSkeleton } from "@/components/ui/loading-skeletons";
import { useMasterKeys, useCreateMasterKey, useRotateMasterKey } from "@/hooks/use-api";
import type { MasterKey } from "@/lib/types";

export const Route = createFileRoute("/keys")({
  component: KeysPage,
});

function KeysPage() {
  return (
    <AuthGuard>
      <MainLayout>
        <MasterKeysPage />
      </MainLayout>
    </AuthGuard>
  );
}

function MasterKeysPage() {
  const { data: masterKeysData, isLoading, error, refetch } = useMasterKeys();
  const createMasterKey = useCreateMasterKey();
  const rotateMasterKey = useRotateMasterKey();
  const { t, i18n } = useTranslation();

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

  const getStatusBadge = (status: MasterKey["status"]) => {
    switch (status) {
      case "Active":
        return (
          <Badge variant="default" className="inline-flex items-center space-x-1">
            <Shield className="h-3 w-3" />
            <span className="text-xs">{t('keys.active')}</span>
          </Badge>
        );
      case "Retired":
        return (
          <Badge variant="secondary" className="inline-flex items-center space-x-1">
            <Clock className="h-3 w-3" />
            <span className="text-xs">{t('keys.retired')}</span>
          </Badge>
        );
      case "Disabled":
        return (
          <Badge variant="destructive" className="inline-flex items-center space-x-1">
            <AlertTriangle className="h-3 w-3" />
            <span className="text-xs">{t('keys.disabled')}</span>
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
    // For now, show coming soon message
    toast.info(t('keys.registerComingSoon'), {
      description: t('keys.registerComingSoonDescription'),
    });
  };

  const handleRotateKey = () => {
    // For now, show coming soon message  
    toast.info(t('keys.rotateComingSoon'), {
      description: t('keys.rotateComingSoonDescription'),  
    });
  };

  if (isLoading) {
    return <MasterKeyListSkeleton />;
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

  const masterKeys = masterKeysData?.master_keys || [];

  return (
    <div className="space-section">
      {/* [顶层] 页面标题 + 主要操作 */}
      <div className="flex items-start justify-between">
        <div className="space-tight">
          <h1 className="text-4xl font-bold tracking-tight">{t('keys.title')}</h1>
          <p className="text-sm text-muted-foreground">
            {t('keys.description')}
          </p>
        </div>
        <div className="flex items-center space-x-2">
          <Button 
            variant="outline" 
            onClick={handleRotateKey}
            disabled={rotateMasterKey.isPending}
            className="border-border"
          >
            <RotateCw className="h-4 w-4 mr-2" />
            {t('keys.rotateKey')}
          </Button>
          <Button 
            onClick={handleRegisterKey}
            disabled={createMasterKey.isPending}
            className="border-border"
          >
            <Plus className="h-4 w-4 mr-2" />
            {t('keys.registerKey')}
          </Button>
        </div>
      </div>

      {/* [中层] 内容分区 - Section卡片 */}
      <Card className="bg-card border border-border">
        {masterKeys.length === 0 ? (
          <div className="padding-section text-center space-content">
            <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-4">
              <Key className="h-6 w-6 text-muted-foreground" />
            </div>
            <p className="text-muted-foreground text-sm">{t('keys.noKeysFound')}</p>
            <p className="text-xs text-muted-foreground mt-2">{t('keys.noKeysFoundDescription')}</p>
            <Button 
              onClick={handleRegisterKey}
              disabled={createMasterKey.isPending}
              variant="outline"
              className="mt-4"
            >
              <Plus className="h-4 w-4 mr-2" />
              {t('keys.registerFirstKey')}
            </Button>
          </div>
        ) : (
          <>
            {/* Mobile Card View */}
            <div className="block md:hidden space-y-4 p-6">
              {masterKeys.map((key: MasterKey) => (
                <div 
                  key={key.id}
                  className="bg-card border border-border rounded-md p-4 space-y-4 hover:bg-accent/50 transition-colors duration-150"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-sm font-medium truncate">
                      {key.id.substring(0, 12)}...
                    </span>
                    {getStatusBadge(key.status)}
                  </div>
                  
                  <div className="space-y-2">
                    <div className="flex justify-between text-xs text-muted-foreground">
                      <span>{t('keys.createdAt')}</span>
                      <span>{formatTimestamp(key.created_at)}</span>
                    </div>
                    
                    {key.description && (
                      <div className="flex justify-between items-start">
                        <span className="text-xs text-muted-foreground">{t('keys.description')}:</span>
                        <span className="text-xs text-right flex-1 ml-2">
                          {key.description}
                        </span>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {/* Desktop Table View */}
            <div className="hidden md:block overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow className="border-b border-border hover:bg-transparent">
                    <TableHead className="font-medium text-foreground">{t('keys.keyId')}</TableHead>
                    <TableHead className="font-medium text-foreground">{t('keys.status')}</TableHead>
                    <TableHead className="font-medium text-foreground hidden lg:table-cell">{t('keys.createdAt')}</TableHead>
                    <TableHead className="font-medium text-foreground hidden xl:table-cell">{t('keys.description')}</TableHead>
                    <TableHead className="font-medium text-foreground text-right">{t('keys.actions')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {masterKeys.map((key: MasterKey) => (
                    <TableRow 
                      key={key.id}
                      className="min-h-12 border-b border-border hover:bg-accent/50 transition-colors duration-150"
                    >
                      <TableCell className="font-medium">
                        <span className="font-mono text-sm" title={key.id}>
                          {key.id.substring(0, 16)}...
                        </span>
                      </TableCell>
                      <TableCell>
                        {getStatusBadge(key.status)}
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground hidden lg:table-cell">
                        {formatTimestamp(key.created_at)}
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground hidden xl:table-cell">
                        {key.description || (
                          <span className="italic text-muted-foreground/60">
                            {t('keys.noDescription')}
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex items-center justify-end space-x-2">
                          <Button 
                            variant="ghost" 
                            size="sm" 
                            disabled
                            className="h-8 w-8 p-0 hover:bg-accent transition-colors duration-150"
                            title={t('keys.viewKey')}
                          >
                            <Key className="h-3 w-3" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </>
        )}
      </Card>

      {/* Info Alert */}
      <Alert className="border-border">
        <Shield className="h-4 w-4" />
        <div className="text-sm">
          <p className="font-medium">{t('keys.securityNote')}</p>
          <p className="text-muted-foreground text-xs mt-1">
            {t('keys.securityNoteDescription')}
          </p>
        </div>
      </Alert>
    </div>
  );
}