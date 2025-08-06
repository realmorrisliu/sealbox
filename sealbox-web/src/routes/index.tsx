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
  Edit,
  Trash2,
  Key,
  Shield,
  Clock,
  AlertTriangle,
  Activity,
  Database,
  Globe,
  Server,
  Zap,
  CheckCircle,
  XCircle,
  RotateCcw,
  Star,
  AlertCircle,
  History,
  LayoutGrid,
  TableIcon,
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
    if (secret.isArchived) return false
    const matchesSearch =
      secret.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      secret.description?.toLowerCase().includes(searchTerm.toLowerCase()) ||
      secret.tags.some((tag) => tag.toLowerCase().includes(searchTerm.toLowerCase()))
    return matchesSearch
  })

  const stats = {
    total: secrets.filter((s) => !s.isArchived).length,
    production: secrets.filter((s) => s.environment === "production" && !s.isArchived).length,
    expiring: secrets.filter((s) => s.status === "expiring" && !s.isArchived).length,
    highRisk: secrets.filter((s) => (s.riskLevel === "high" || s.riskLevel === "critical") && !s.isArchived).length,
    needsRotation: secrets.filter((s) => {
      if (s.isArchived) return false
      if (!s.lastRotated) return true
      const lastRotated = new Date(s.lastRotated)
      const ninetyDaysAgo = new Date()
      ninetyDaysAgo.setDate(ninetyDaysAgo.getDate() - 90)
      return lastRotated <= ninetyDaysAgo
    }).length,
  }


  const toggleFavorite = (secretId: string) => {
    setSecrets(secrets.map((s) => (s.id === secretId ? { ...s, isFavorite: !s.isFavorite } : s)))
  }

  const showDecryptHint = (secretName: string) => {
    toast.info(t('secrets.decryptHint.title'), {
      description: t('secrets.decryptHint.description', { 
        command: `sealbox-cli secret get ${secretName}` 
      }),
      duration: 5000,
    });
  }

  const handleDeleteSecret = async (secret: any) => {
    if (!window.confirm(t('secrets.confirmDelete', { name: secret.name }))) {
      return;
    }

    try {
      // Get the actual version number from the API data
      const apiSecret = secretsData?.secrets.find(s => s.key === secret.name);
      if (!apiSecret) {
        toast.error(t('common.error'), {
          description: 'Secret not found'
        });
        return;
      }

      await deleteSecretMutation.mutateAsync({
        key: secret.name,
        version: apiSecret.version
      });

      toast.success(t('secrets.deleted'), {
        description: t('secrets.deletedDescription', { name: secret.name })
      });
    } catch (error: any) {
      toast.error(t('common.error'), {
        description: error.message || 'Failed to delete secret'
      });
    }
  }

  const getEnvironmentColor = (env: string) => {
    switch (env) {
      case "production":
        return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
      case "staging":
        return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200"
      case "development":
        return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
      default:
        return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200"
    }
  }

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "database":
        return <Database className="w-3 h-3" />
      case "api":
        return <Globe className="w-3 h-3" />
      case "auth":
        return <Shield className="w-3 h-3" />
      case "payment":
        return <Zap className="w-3 h-3" />
      case "service":
        return <Server className="w-3 h-3" />
      default:
        return <Key className="w-3 h-3" />
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
      case "compromised":
        return "text-red-700"
      case "inactive":
        return "text-gray-500"
      default:
        return "text-gray-600"
    }
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "active":
        return <CheckCircle className="h-3 w-3" />
      case "expiring":
        return <AlertTriangle className="h-3 w-3" />
      case "expired":
        return <XCircle className="h-3 w-3" />
      case "compromised":
        return <AlertCircle className="h-3 w-3" />
      case "inactive":
        return <XCircle className="h-3 w-3" />
      default:
        return <CheckCircle className="h-3 w-3" />
    }
  }

  const getRiskLevelColor = (riskLevel: string) => {
    switch (riskLevel) {
      case "critical":
        return "text-red-700"
      case "high":
        return "text-red-600"
      case "medium":
        return "text-yellow-600"
      case "low":
        return "text-green-600"
      default:
        return "text-gray-600"
    }
  }

  if (isLoading) {
    return <div>{t('common.loading')}</div>
  }

  if (error) {
    return <div>{t('common.error')}</div>
  }

  return (
    <div className="w-full px-2 py-4 space-y-4">
      {/* Stats Cards */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-2">
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
              <p className="text-xs text-muted-foreground">{t('secrets.stats.production')}</p>
              <p className="text-lg font-bold">{stats.production}</p>
            </div>
            <Shield className="w-4 h-4 text-red-500" />
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
              <p className="text-xs text-muted-foreground">{t('secrets.stats.highRisk')}</p>
              <p className="text-lg font-bold">{stats.highRisk}</p>
            </div>
            <AlertCircle className="w-4 h-4 text-red-500" />
          </div>
        </Card>
        <Card className="p-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs text-muted-foreground">{t('secrets.stats.needRotation')}</p>
              <p className="text-lg font-bold">{stats.needsRotation}</p>
            </div>
            <RotateCcw className="w-4 h-4 text-orange-500" />
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
                <TableHead className="w-4 h-10 px-3"></TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.name')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.value')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.environment')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.category')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.status')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.riskLevel')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.versions')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.ttl')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.lastUsed')}</TableHead>
                <TableHead className="h-10 px-3 font-semibold">{t('secrets.table.access')}</TableHead>
                <TableHead className="w-24 h-10 px-3 font-semibold">{t('secrets.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredSecrets.map((secret) => (
                <TableRow key={secret.id} className="text-xs hover:bg-muted/20 transition-colors">
                  <TableCell className="px-3 py-3">
                    <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={() => toggleFavorite(secret.id)}>
                      <Star
                        className={`h-3 w-3 ${secret.isFavorite ? "fill-yellow-400 text-yellow-400" : "text-gray-400"}`}
                      />
                    </Button>
                  </TableCell>
                  <TableCell className="px-3 py-3 font-medium">
                    <div className="flex items-center gap-2">
                      {getCategoryIcon(secret.category)}
                      <span className="font-mono">{secret.name}</span>
                    </div>
                    {secret.description && (
                      <div className="text-muted-foreground text-xs mt-1">{secret.description}</div>
                    )}
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <div className="flex items-center gap-1 max-w-48">
                      <code className="text-xs bg-muted px-1 py-0.5 rounded font-mono text-muted-foreground">
                        {secret.value}
                      </code>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 w-6 p-0"
                        onClick={() => showDecryptHint(secret.name)}
                        title={t('secrets.decryptHint.tooltip')}
                      >
                        <Eye className="h-3 w-3" />
                      </Button>
                    </div>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <Badge className={`text-xs px-1.5 py-0.5 ${getEnvironmentColor(secret.environment)}`}>
                      {secret.environment.charAt(0).toUpperCase() + secret.environment.slice(1, 4)}
                    </Badge>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <div className="flex items-center gap-1">
                      {getCategoryIcon(secret.category)}
                      <span className="capitalize">{t(`secrets.category.${secret.category}`)}</span>
                    </div>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <div className={`flex items-center gap-1 ${getStatusColor(secret.status)}`}>
                      {getStatusIcon(secret.status)}
                      <span className="capitalize">{t(`secrets.status.${secret.status}`)}</span>
                    </div>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <div className={`flex items-center gap-1 ${getRiskLevelColor(secret.riskLevel)}`}>
                      <span className="capitalize">{t(`secrets.riskLevel.${secret.riskLevel}`)}</span>
                    </div>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 px-2 text-xs"
                    >
                      <History className="h-3 w-3 mr-1" />v{secret.versions.length}
                    </Button>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    {secret.expiresAt ? (
                      <CountdownTimer
                        expiresAt={new Date(secret.expiresAt).getTime() / 1000}
                        className="text-xs"
                      />
                    ) : (
                      <span className="text-xs text-muted-foreground">{t('common.noExpiry')}</span>
                    )}
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <div className="flex items-center gap-1 text-muted-foreground">
                      <Clock className="h-3 w-3" />
                      {secret.lastUsed || t('secrets.cardView.never')}
                    </div>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <div className="flex items-center gap-1 text-muted-foreground">
                      <Activity className="h-3 w-3" />
                      {secret.accessCount}
                    </div>
                  </TableCell>
                  <TableCell className="px-3 py-3">
                    <div className="flex items-center gap-1">
                      <Button 
                        variant="ghost" 
                        size="sm" 
                        className="h-6 w-6 p-0"
                        disabled
                        title={t('secrets.editHint')}
                      >
                        <Edit className="h-3 w-3" />
                      </Button>
                      <Button 
                        variant="ghost" 
                        size="sm" 
                        className="h-6 w-6 p-0 text-red-600"
                        onClick={() => handleDeleteSecret(secret)}
                        disabled={deleteSecretMutation.isPending}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
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
            <Card key={secret.id} className="p-4">
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-2">
                    <Button variant="ghost" size="sm" className="h-5 w-5 p-0" onClick={() => toggleFavorite(secret.id)}>
                      <Star
                        className={`h-3 w-3 ${secret.isFavorite ? "fill-yellow-400 text-yellow-400" : "text-gray-400"}`}
                      />
                    </Button>
                    {getCategoryIcon(secret.category)}
                    <h3 className="font-mono font-medium">{secret.name}</h3>
                    <Badge className={`text-xs px-1.5 py-0.5 ${getEnvironmentColor(secret.environment)}`}>
                      {secret.environment.charAt(0).toUpperCase() + secret.environment.slice(1, 4)}
                    </Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 px-2 text-xs"
                    >
                      <History className="h-3 w-3 mr-1" />v{secret.versions.length}
                    </Button>
                  </div>

                  {secret.description && <p className="text-sm text-muted-foreground mb-2">{secret.description}</p>}

                  <div className="flex items-center gap-2 mb-2">
                    <code className="bg-muted px-2 py-1 rounded text-xs font-mono flex-1 min-w-0 truncate text-muted-foreground">
                      {secret.value}
                    </code>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0"
                      onClick={() => showDecryptHint(secret.name)}
                      title={t('secrets.decryptHint.tooltip')}
                    >
                      <Eye className="h-3 w-3" />
                    </Button>
                  </div>

                  <div className="flex items-center gap-4 text-xs text-muted-foreground">
                    <div className={`flex items-center gap-1 ${getStatusColor(secret.status)}`}>
                      {getStatusIcon(secret.status)}
                      <span>{secret.status}</span>
                    </div>
                    <div className={`flex items-center gap-1 ${getRiskLevelColor(secret.riskLevel)}`}>
                      <span>{t(`secrets.riskLevel.${secret.riskLevel}`)} {t('secrets.cardView.risk')}</span>
                    </div>
                    {secret.expiresAt && (
                      <div className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        <CountdownTimer
                          expiresAt={new Date(secret.expiresAt).getTime() / 1000}
                          className="text-xs"
                        />
                      </div>
                    )}
                    <div className="flex items-center gap-1">
                      <Activity className="h-3 w-3" />
                      {secret.accessCount} {t('secrets.cardView.uses')}
                    </div>
                    <div className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {secret.lastUsed || t('secrets.cardView.never')}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-1 ml-4">
                  <Button 
                    variant="ghost" 
                    size="sm" 
                    className="h-7 w-7 p-0"
                    disabled
                    title={t('secrets.editHint')}
                  >
                    <Edit className="h-3 w-3" />
                  </Button>
                  <Button 
                    variant="ghost" 
                    size="sm" 
                    className="h-7 w-7 p-0 text-red-600"
                    onClick={() => handleDeleteSecret(secret)}
                    disabled={deleteSecretMutation.isPending}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}

      {filteredSecrets.length === 0 && (
        <Card className="p-8">
          <div className="text-center">
            <Key className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">{t('secrets.empty.noSecretsFound')}</h3>
            <p className="text-muted-foreground mb-4">
              {searchTerm ? t('secrets.empty.tryAdjustingFilters') : t('secrets.empty.addFirstSecret')}
            </p>
            <CreateSecretDialog>
              <Button>
                <Plus className="h-4 w-4 mr-2" />
                {t('secrets.controls.addSecret')}
              </Button>
            </CreateSecretDialog>
          </div>
        </Card>
      )}
    </div>
  )
}
