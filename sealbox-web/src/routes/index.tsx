"use client"

import { createFileRoute } from "@tanstack/react-router";
import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  Plus,
  Search,
  Eye,
  Trash2,
  Key,
  Clock,
  AlertTriangle,
  TableIcon,
  LayoutGrid,
} from "lucide-react"
import { AuthGuard } from "@/components/auth/auth-guard";
import { MainLayout } from "@/components/layout/main-layout";
import { CreateSecretDialog } from "@/components/secrets/create-secret-dialog";
import { CountdownTimer } from "@/components/common/countdown-timer";
import { useSecrets, useDeleteSecret } from "@/hooks/use-api";
import { toast } from "sonner";
import type { SecretUIData } from "@/lib/types";
import { convertSecretToUIData } from "@/lib/types";
import { useTranslation } from "react-i18next";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <AuthGuard>
      <MainLayout>
        <SecretManagement />
      </MainLayout>
    </AuthGuard>
  );
}

function SecretManagement() {
  const { t } = useTranslation()
  const { data: secretsData, isLoading, error } = useSecrets();
  const deleteSecretMutation = useDeleteSecret();
  const [secrets, setSecrets] = useState<SecretUIData[]>([])
  const [searchTerm, setSearchTerm] = useState("")
  const [viewMode, setViewMode] = useState<"table" | "cards">("table")

  // Convert API data to display format
  useEffect(() => {
    if (secretsData?.secrets) {
      const convertedSecrets = secretsData.secrets.map(convertSecretToUIData)
      setSecrets(convertedSecrets)
    }
  }, [secretsData])

  const filteredSecrets = secrets.filter((secret) => {
    return secret.key.toLowerCase().includes(searchTerm.toLowerCase())
  })

  const stats = {
    total: secrets.length,
    expiring: secrets.filter((s) => s.status === "expiring").length,
    expired: secrets.filter((s) => s.status === "expired").length,
  }

  const showDecryptHint = (secretName: string) => {
    toast.info(t('secrets.decryptHint.title'), {
      description: t('secrets.decryptHint.description', { 
        command: `sealbox-cli secret get ${secretName}` 
      }),
      duration: 5000,
    });
  }

  const handleDeleteSecret = async (secret: SecretUIData) => {
    if (!window.confirm(t('secrets.confirmDelete', { name: secret.key }))) {
      return;
    }

    try {
      await deleteSecretMutation.mutateAsync({
        key: secret.key,
        version: secret.version
      });

      toast.success(t('secrets.deleted'), {
        description: t('secrets.deletedDescription', { name: secret.key })
      });
    } catch (error: any) {
      toast.error(t('common.error'), {
        description: error.message || 'Failed to delete secret'
      });
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case "active":
        return "text-green-600"
      case "expiring":
        return "text-yellow-600"
      case "expired":
        return "text-red-600"
      default:
        return "text-gray-600"
    }
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "active":
        return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
      case "expiring":
        return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200"
      case "expired":
        return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
      default:
        return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200"
    }
  }

  if (isLoading) {
    return <div className="flex items-center justify-center p-8">{t('common.loading')}</div>
  }

  if (error) {
    return <div className="flex items-center justify-center p-8 text-red-500">{t('common.error')}</div>
  }

  return (
    <div className="w-full px-2 py-4 space-y-4">
      {/* Stats Cards */}
      <div className="grid grid-cols-3 gap-2">
        <Card className="p-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs text-muted-foreground">{t('secrets.stats.totalSecrets')}</p>
              <p className="text-lg font-bold">{stats.total}</p>
            </div>
            <Key className="w-4 h-4 text-blue-500" />
          </div>
        </Card>
        <Card className="p-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs text-muted-foreground">{t('secrets.stats.expiring')}</p>
              <p className="text-lg font-bold">{stats.expiring}</p>
            </div>
            <AlertTriangle className="w-4 h-4 text-yellow-500" />
          </div>
        </Card>
        <Card className="p-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs text-muted-foreground">{t('secrets.stats.expired')}</p>
              <p className="text-lg font-bold">{stats.expired}</p>
            </div>
            <Clock className="w-4 h-4 text-red-500" />
          </div>
        </Card>
      </div>

      {/* Controls */}
      <Card className="p-3">
        <div className="flex flex-col md:flex-row gap-3 items-start md:items-center justify-between">
          <div className="flex flex-col sm:flex-row gap-2 flex-1">
            <div className="relative">
              <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-3 w-3 text-muted-foreground" />
              <Input
                placeholder={t('secrets.controls.searchPlaceholder')}
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="pl-7 h-8 w-64"
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Tabs value={viewMode} onValueChange={(v) => setViewMode(v as "table" | "cards")}>
              <TabsList className="h-8">
                <TabsTrigger value="table" className="px-2" title={t('secrets.controls.table')}>
                  <TableIcon className="h-4 w-4" />
                </TabsTrigger>
                <TabsTrigger value="cards" className="px-2" title={t('secrets.controls.cards')}>
                  <LayoutGrid className="h-4 w-4" />
                </TabsTrigger>
              </TabsList>
            </Tabs>

            <CreateSecretDialog>
              <Button size="sm" className="h-8">
                <Plus className="h-3 w-3 mr-1" />
                {t('secrets.controls.addSecret')}
              </Button>
            </CreateSecretDialog>
          </div>
        </div>
      </Card>

      {/* Secrets Table */}
      {viewMode === "table" ? (
        <Card className="overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow className="text-xs border-b bg-muted/30">
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.name')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.version')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.status')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.created')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.updated')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.ttl')}</TableHead>
                <TableHead className="w-24 h-10 px-3 font-semibold">{t('secrets.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredSecrets.map((secret) => (
                <TableRow key={`${secret.key}-${secret.version}`} className="text-xs hover:bg-muted/20 transition-colors">
                  <TableCell className="px-3 py-3 font-medium">
                    <div className="flex items-center gap-2">
                      <span className="font-mono">{secret.key}</span>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 w-6 p-0"
                        onClick={() => showDecryptHint(secret.key)}
                        title={t('secrets.decryptHint.tooltip')}
                      >
                        <Eye className="h-3 w-3" />
                      </Button>
                    </div>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <span className="font-mono">v{secret.version}</span>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <Badge className={`text-xs px-1.5 py-0.5 ${getStatusBadge(secret.status)}`}>
                      {t(`secrets.status.${secret.status}`)}
                    </Badge>
                  </TableCell>
                  <TableCell className="px-3 py-3 text-muted-foreground">
                    {secret.createdAt}
                  </TableCell>
                  <TableCell className="px-3 py-3 text-muted-foreground">
                    {secret.updatedAt}
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    {secret.expires_at ? (
                      <CountdownTimer
                        expiresAt={secret.expires_at}
                        className="text-xs"
                      />
                    ) : (
                      <span className="text-xs text-muted-foreground">{t('common.noExpiry')}</span>
                    )}
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <Button 
                      variant="ghost" 
                      size="sm" 
                      className="h-6 w-6 p-0 text-red-600"
                      onClick={() => handleDeleteSecret(secret)}
                      disabled={deleteSecretMutation.isPending}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      ) : (
        /* Cards View */
        <div className="grid gap-3">
          {filteredSecrets.map((secret) => (
            <Card key={`${secret.key}-${secret.version}`} className="p-4">
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-2">
                    <h3 className="font-mono font-medium">{secret.key}</h3>
                    <Badge className={`text-xs px-1.5 py-0.5 ${getStatusBadge(secret.status)}`}>
                      {t(`secrets.status.${secret.status}`)}
                    </Badge>
                    <span className="text-xs text-muted-foreground">v{secret.version}</span>
                  </div>

                  <div className="flex items-center gap-2 mb-2">
                    <code className="bg-muted px-2 py-1 rounded text-xs font-mono flex-1 min-w-0 truncate text-muted-foreground">
                      [ENCRYPTED]
                    </code>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0"
                      onClick={() => showDecryptHint(secret.key)}
                      title={t('secrets.decryptHint.tooltip')}
                    >
                      <Eye className="h-3 w-3" />
                    </Button>
                  </div>

                  <div className="flex items-center gap-4 text-xs text-muted-foreground">
                    <span>{t('secrets.table.created')}: {secret.createdAt}</span>
                    <span>{t('secrets.table.updated')}: {secret.updatedAt}</span>
                    {secret.expires_at && (
                      <div className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        <CountdownTimer expiresAt={secret.expires_at} className="text-xs" />
                      </div>
                    )}
                  </div>
                </div>

                <Button 
                  variant="ghost" 
                  size="sm" 
                  className="h-8 w-8 p-0 text-red-600"
                  onClick={() => handleDeleteSecret(secret)}
                  disabled={deleteSecretMutation.isPending}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  )
}