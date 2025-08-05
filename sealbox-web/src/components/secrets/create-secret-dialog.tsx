import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Plus, X, Clock, Info } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Alert } from "@/components/ui/alert";
import { useCreateSecret } from "@/hooks/use-api";

// Form validation schema
const createSecretSchema = z.object({
  key: z.string()
    .min(1, "Secret name is required")
    .max(255, "Secret name must be less than 255 characters")
    .regex(/^[a-zA-Z0-9_-]+$/, "Secret name can only contain letters, numbers, underscores and hyphens"),
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

type CreateSecretForm = z.infer<typeof createSecretSchema>;

interface CreateSecretDialogProps {
  children?: React.ReactNode;
}

export function CreateSecretDialog({ children }: CreateSecretDialogProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const createSecret = useCreateSecret();

  const form = useForm<CreateSecretForm>({
    resolver: zodResolver(createSecretSchema),
    defaultValues: {
      key: "",
      secret: "",
      hasTtl: false,
      ttlValue: undefined,
      ttlUnit: "hours",
    },
  });

  const watchHasTtl = form.watch("hasTtl");

  const onSubmit = async (data: CreateSecretForm) => {
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
        key: data.key,
        data: {
          secret: data.secret,
          ttl,
        },
      });

      toast.success(t('secrets.createSuccess'), {
        description: t('secrets.createSuccessDescription', { key: data.key }),
      });

      // Close dialog and form will be reset by useEffect
      setOpen(false);
    } catch (error) {
      console.error("Create secret failed:", error);
      toast.error(t('secrets.createFailed'), {
        description: t('secrets.createFailedDescription'),
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
          <Button className="border-border">
            <Plus className="h-4 w-4 mr-2" />
            {t('secrets.newSecret')}
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-[500px] bg-card border-border">
        <DialogHeader className="space-tight">
          <div className="flex items-center justify-between">
            <DialogTitle className="text-xl font-semibold">
              {t('secrets.createSecretTitle')}
            </DialogTitle>
          </div>
          <DialogDescription className="text-sm text-muted-foreground">
            {t('secrets.createSecretDescription')}
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-content">
            {/* Secret Name Field */}
            <FormField
              control={form.control}
              name="key"
              render={({ field }) => (
                <FormItem className="space-tight">
                  <FormLabel className="text-sm font-medium">
                    {t('secrets.secretName')}
                  </FormLabel>
                  <FormControl>
                    <Input
                      placeholder={t('secrets.secretNamePlaceholder')}
                      className="border-border"
                      {...field}
                    />
                  </FormControl>
                  <FormDescription className="text-xs text-muted-foreground">
                    {t('secrets.secretNameHelp')}
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

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
                <p className="font-medium">{t('secrets.encryptionInfo')}</p>
                <p className="text-muted-foreground text-xs mt-1">
                  {t('secrets.encryptionInfoDescription')}
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
                    <span>{t('secrets.creating')}</span>
                  </div>
                ) : (
                  <div className="flex items-center space-x-2">
                    <Plus className="h-4 w-4" />
                    <span>{t('secrets.createSecret')}</span>
                  </div>
                )}
              </Button>
            </div>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}