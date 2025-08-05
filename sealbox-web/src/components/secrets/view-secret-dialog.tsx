import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Eye, Copy, EyeOff, Clock, AlertTriangle, Info, Loader2 } from "lucide-react";
import { format } from "date-fns";
import { enUS, zhCN, ja, de } from "date-fns/locale";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Alert } from "@/components/ui/alert";
import { Skeleton } from "@/components/ui/skeleton";
import { useSecret } from "@/hooks/use-api";
import type { SecretInfo } from "@/lib/types";

interface ViewSecretDialogProps {
  secret: SecretInfo;
  children?: React.ReactNode;
}

export function ViewSecretDialog({ secret, children }: ViewSecretDialogProps) {
  const { t, i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  const [showSecret, setShowSecret] = useState(false);
  
  const { data: secretData, isLoading, error, refetch } = useSecret(
    secret.key, 
    secret.version,
    { enabled: open } // Only fetch when dialog is open
  );

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

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(t('secrets.copiedToClipboard'), {
        description: t('secrets.copiedToClipboardDescription'),
      });
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
      toast.error(t('secrets.copyFailed'), {
        description: t('secrets.copyFailedDescription'),
      });
    }
  };

  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen) {
      setShowSecret(false); // Hide secret when closing dialog
    }
    setOpen(newOpen);
  };

  const expiryStatus = getExpiryStatus(secret.expires_at);

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        {children || (
          <Button variant="ghost" size="sm" className="h-8 w-8 p-0 hover:bg-accent">
            <Eye className="h-3 w-3" />
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-[600px] bg-card border-border">
        <DialogHeader className="space-tight">
          <div className="flex items-center justify-between">
            <DialogTitle className="text-xl font-semibold">
              {t('secrets.viewSecretTitle')}
            </DialogTitle>
            <Badge variant="secondary" className="font-mono text-xs">
              v{secret.version}
            </Badge>
          </div>
          <DialogDescription className="text-sm text-muted-foreground">
            {t('secrets.viewSecretDescription')}
          </DialogDescription>
        </DialogHeader>

        <div className="space-content">
          {/* Secret Metadata */}
          <div className="space-tight">
            <h3 className="text-sm font-medium mb-2">{t('secrets.secretDetails')}</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-muted-foreground">{t('secrets.secretName')}:</span>
                <div className="font-mono mt-1 break-all">{secret.key}</div>
              </div>
              <div>
                <span className="text-muted-foreground">{t('secrets.version')}:</span>
                <div className="mt-1">v{secret.version}</div>
              </div>
              <div>
                <span className="text-muted-foreground">{t('secrets.createdAt')}:</span>
                <div className="mt-1">{formatTimestamp(secret.created_at)}</div>
              </div>
              <div>
                <span className="text-muted-foreground">{t('secrets.updatedAt')}:</span>
                <div className="mt-1">{formatTimestamp(secret.updated_at)}</div>
              </div>
            </div>
          </div>

          {/* Expiry Status */}
          <div className="space-tight">
            <span className="text-sm text-muted-foreground">{t('secrets.expiresAt')}:</span>
            <div className="mt-1">
              {secret.expires_at ? (
                <Badge 
                  variant={
                    expiryStatus?.status === "expired" ? "destructive" :
                    expiryStatus?.status === "warning" ? "default" : "secondary"
                  }
                  className="inline-flex items-center space-x-1"
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

          {/* Secret Content */}
          <div className="space-tight">
            <div className="flex items-center justify-between mb-2">
              <h3 className="text-sm font-medium">{t('secrets.secretContent')}</h3>
              <div className="flex items-center space-x-2">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowSecret(!showSecret)}
                  className="h-8"
                >
                  {showSecret ? (
                    <>
                      <EyeOff className="h-3 w-3 mr-1" />
                      {t('secrets.hideSecret')}
                    </>
                  ) : (
                    <>
                      <Eye className="h-3 w-3 mr-1" />
                      {t('secrets.showSecret')}
                    </>
                  )}
                </Button>
                {secretData?.secret && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => copyToClipboard(secretData.secret)}
                    disabled={!showSecret}
                    className="h-8"
                  >
                    <Copy className="h-3 w-3 mr-1" />
                    {t('common.copy')}
                  </Button>
                )}
              </div>
            </div>

            <div className="border border-border rounded-md bg-muted/30 p-4 min-h-[100px]">
              {isLoading ? (
                <div className="space-y-2">
                  <Skeleton className="h-4 w-full" />
                  <Skeleton className="h-4 w-3/4" />
                  <Skeleton className="h-4 w-1/2" />
                </div>
              ) : error ? (
                <Alert variant="destructive">
                  <AlertTriangle className="h-4 w-4" />
                  <div>
                    <p className="font-medium">{t('secrets.loadSecretFailed')}</p>
                    <p className="text-sm">{error.message}</p>
                    <Button 
                      variant="outline" 
                      size="sm" 
                      onClick={() => refetch()}
                      className="mt-2"
                    >
                      {t('common.retry')}
                    </Button>
                  </div>
                </Alert>
              ) : secretData ? (
                <div className="font-mono text-sm">
                  {showSecret ? (
                    <pre className="whitespace-pre-wrap break-words">
                      {secretData.secret}
                    </pre>
                  ) : (
                    <div className="text-muted-foreground italic">
                      {t('secrets.secretHidden')}
                    </div>
                  )}
                </div>
              ) : (
                <div className="text-muted-foreground italic text-sm">
                  {t('secrets.noSecretData')}
                </div>
              )}
            </div>
          </div>

          {/* Security Info */}
          <Alert className="border-border">
            <Info className="h-4 w-4" />
            <div className="text-sm">
              <p className="font-medium">{t('secrets.securityInfo')}</p>
              <p className="text-muted-foreground text-xs mt-1">
                {t('secrets.securityInfoDescription')}
              </p>
            </div>
          </Alert>

          {/* Action Buttons */}
          <div className="flex items-center justify-end space-x-2 pt-4 border-t border-border">
            <Button
              variant="outline"
              onClick={() => setOpen(false)}
              className="border-border"
            >
              {t('common.close')}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}