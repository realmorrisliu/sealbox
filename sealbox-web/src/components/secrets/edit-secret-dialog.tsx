import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Edit, X, Clock, Info, Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { useSecret, useCreateSecret } from "@/hooks/use-api";
import type { SecretInfo } from "@/lib/types";

// Form validation schema (same as create, but with existing data)
const editSecretSchema = z.object({
  secret: z.string()
    .min(1, "Secret value is required")
    .max(10000, "Secret value must be less than 10,000 characters"),
  hasTtl: z.boolean().default(false),
  ttlValue: z.number().int().positive().optional(),
  ttlUnit: z.enum(["minutes", "hours", "days"]).default("hours"),
}).refine((data) => {
  if (data.hasTtl && !data.ttlValue) {
    return false;
  }
  return true;
}, {
  message: "TTL value is required when expiration is enabled",
  path: ["ttlValue"]
});

type EditSecretForm = z.infer<typeof editSecretSchema>;

interface EditSecretDialogProps {
  secret: SecretInfo;
  children?: React.ReactNode;
}

export function EditSecretDialog({ secret, children }: EditSecretDialogProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const createSecret = useCreateSecret(); // We use createSecret to create a new version
  
  const { data: secretData, isLoading, error, refetch } = useSecret(
    secret.key, 
    secret.version,
    { enabled: open } // Only fetch when dialog is open
  );

  const form = useForm<EditSecretForm>({
    resolver: zodResolver(editSecretSchema),
    defaultValues: {
      secret: "",
      hasTtl: false,
      ttlValue: undefined,
      ttlUnit: "hours",
    },
  });

  const watchHasTtl = form.watch("hasTtl");

  // Load existing secret data into form when available
  useEffect(() => {
    if (secretData && open) {
      form.reset({
        secret: secretData.secret,
        hasTtl: !!secret.expires_at,
        ttlValue: secret.expires_at ? Math.ceil((secret.expires_at - Date.now() / 1000) / 3600) : undefined,
        ttlUnit: "hours",
      });
    }
  }, [secretData, secret, open, form]);

  const onSubmit = async (data: EditSecretForm) => {
    try {
      let ttl: number | undefined;
      
      if (data.hasTtl && data.ttlValue) {
        // Convert TTL to seconds based on unit
        const multipliers = {
          minutes: 60,
          hours: 3600,
          days: 86400,
        };
        ttl = data.ttlValue * multipliers[data.ttlUnit];
      }

      await createSecret.mutateAsync({
        key: secret.key,
        data: {
          secret: data.secret,
          ttl,
        },
      });

      toast.success(t('secrets.updateSuccess'), {
        description: t('secrets.updateSuccessDescription', { key: secret.key }),
      });

      // Close dialog and form will be reset by useEffect
      setOpen(false);
    } catch (error) {
      console.error("Update secret failed:", error);
      toast.error(t('secrets.updateFailed'), {
        description: t('secrets.updateFailedDescription'),
      });
    }
  };

  // Reset form when dialog closes
  useEffect(() => {
    if (!open) {
      form.reset();
    }
  }, [open, form]);

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        {children || (
          <Button variant="ghost" size="sm" className="h-8 w-8 p-0 hover:bg-accent">
            <Edit className="h-3 w-3" />
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-[500px] bg-card border-border">
        <DialogHeader className="space-tight">
          <div className="flex items-center justify-between">
            <DialogTitle className="text-xl font-semibold">
              {t('secrets.editSecretTitle')}
            </DialogTitle>
            <Badge variant="secondary" className="font-mono text-xs">
              v{secret.version} → {secret.version + 1}
            </Badge>
          </div>
          <DialogDescription className="text-sm text-muted-foreground">
            {t('secrets.editSecretDescription')}
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="space-content">
            <div className="space-y-4">
              <div className="space-y-2">
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-10 w-full" />
              </div>
              <div className="space-y-2">
                <Skeleton className="h-4 w-24" />
                <Skeleton className="h-20 w-full" />
              </div>
              <Skeleton className="h-16 w-full" />
            </div>
          </div>
        ) : error ? (
          <div className="space-content">
            <Alert variant="destructive">
              <Info className="h-4 w-4" />
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
          </div>
        ) : (
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="space-content">
              {/* Secret Name (Read-only) */}
              <div className="space-tight">
                <label className="text-sm font-medium text-muted-foreground">
                  {t('secrets.secretName')}
                </label>
                <div className="flex h-10 w-full rounded-md border border-border bg-muted px-3 py-2">
                  <span className="font-mono text-sm text-muted-foreground">{secret.key}</span>
                </div>
                <p className="text-xs text-muted-foreground">
                  {t('secrets.secretNameReadonly')}
                </p>
              </div>

              {/* Secret Value Field */}
              <FormField
                control={form.control}
                name="secret"
                render={({ field }) => (
                  <FormItem className="space-tight">
                    <FormLabel className="text-sm font-medium">
                      {t('secrets.secretValue')}
                    </FormLabel>
                    <FormControl>
                      <textarea
                        placeholder={t('secrets.secretValuePlaceholder')}
                        className="flex min-h-[80px] w-full rounded-md border border-border bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 resize-none"
                        {...field}
                      />
                    </FormControl>
                    <FormDescription className="text-xs text-muted-foreground">
                      {t('secrets.secretValueHelp')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              {/* TTL Configuration */}
              <div className="space-tight">
                <FormField
                  control={form.control}
                  name="hasTtl"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-center justify-between rounded-lg border border-border p-4 space-y-0">
                      <div className="space-y-0.5">
                        <FormLabel className="text-sm font-medium">
                          {t('secrets.enableTtl')}
                        </FormLabel>
                        <FormDescription className="text-xs text-muted-foreground">
                          {t('secrets.enableTtlDescription')}
                        </FormDescription>
                      </div>
                      <FormControl>
                        <Switch
                          checked={field.value}
                          onCheckedChange={field.onChange}
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />

                {watchHasTtl && (
                  <div className="grid grid-cols-2 gap-4 pt-2">
                    <FormField
                      control={form.control}
                      name="ttlValue"
                      render={({ field }) => (
                        <FormItem className="space-tight">
                          <FormLabel className="text-sm font-medium">
                            {t('secrets.ttlValue')}
                          </FormLabel>
                          <FormControl>
                            <Input
                              type="number"
                              min="1"
                              placeholder="1"
                              className="border-border"
                              {...field}
                              onChange={(e) => field.onChange(e.target.value ? parseInt(e.target.value) : undefined)}
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    <FormField
                      control={form.control}
                      name="ttlUnit"
                      render={({ field }) => (
                        <FormItem className="space-tight">
                          <FormLabel className="text-sm font-medium">
                            {t('secrets.ttlUnit')}
                          </FormLabel>
                          <FormControl>
                            <select
                              className="flex h-10 w-full rounded-md border border-border bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                              {...field}
                            >
                              <option value="minutes">{t('secrets.minutes')}</option>
                              <option value="hours">{t('secrets.hours')}</option>
                              <option value="days">{t('secrets.days')}</option>
                            </select>
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                  </div>
                )}
              </div>

              {/* Info Alert */}
              <Alert className="border-border">
                <Info className="h-4 w-4" />
                <div className="text-sm">
                  <p className="font-medium">{t('secrets.versioningInfo')}</p>
                  <p className="text-muted-foreground text-xs mt-1">
                    {t('secrets.versioningInfoDescription')}
                  </p>
                </div>
              </Alert>

              {/* Action Buttons */}
              <div className="flex items-center justify-end space-x-2 pt-4 border-t border-border">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setOpen(false)}
                  disabled={createSecret.isPending}
                  className="border-border"
                >
                  {t('common.cancel')}
                </Button>
                <Button
                  type="submit"
                  disabled={createSecret.isPending}
                  className="min-w-[100px]"
                >
                  {createSecret.isPending ? (
                    <div className="flex items-center space-x-2">
                      <div className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin" />
                      <span>{t('secrets.updating')}</span>
                    </div>
                  ) : (
                    <div className="flex items-center space-x-2">
                      <Edit className="h-4 w-4" />
                      <span>{t('secrets.updateSecret')}</span>
                    </div>
                  )}
                </Button>
              </div>
            </form>
          </Form>
        )}
      </DialogContent>
    </Dialog>
  );
}