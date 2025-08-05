import { createFileRoute, useRouter } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert } from "@/components/ui/alert";
import { LanguageSelector } from "@/components/i18n/language-selector";
import { SealboxIcon } from "@/components/brand/sealbox-logo";
import { useAuthStore } from "@/stores/auth";
import { useConfigStore } from "@/stores/config";
import { createApiClient } from "@/lib/api";

type LoginForm = {
  serverUrl: string;
  token: string;
};

export const Route = createFileRoute("/login")({
  component: LoginPage,
});

function LoginPage() {
  const router = useRouter();
  const { login, isAuthenticated } = useAuthStore();
  const { defaultServerUrl, setDefaultServerUrl } = useConfigStore();
  const { t } = useTranslation();
  
  const loginSchema = z.object({
    serverUrl: z.string().url(t('login.errors.invalidUrl')),
    token: z.string().min(1, t('login.errors.tokenRequired')),
  });
  
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
    setValue,
    setError,
  } = useForm<LoginForm>({
    resolver: zodResolver(loginSchema),
    defaultValues: {
      serverUrl: defaultServerUrl,
      token: "",
    },
  });

  // If already authenticated, redirect to home page
  useEffect(() => {
    if (isAuthenticated) {
      router.navigate({ to: "/" });
    }
  }, [isAuthenticated, router]);

  const onSubmit = async (data: LoginForm) => {
    try {
      // Test connection - use readiness endpoint to verify server status and authentication
      const api = createApiClient(data.serverUrl, data.token);
      await api.readiness();
      
      // Connection successful, save authentication info
      login(data.token, data.serverUrl);
      setDefaultServerUrl(data.serverUrl);
      
      // Navigate to home page
      router.navigate({ to: "/" });
    } catch (error: any) {
      console.error("Login failed:", error);
      
      if (error.status === 401 || error.status === 403) {
        setError("token", { message: t('login.errors.authFailed') });
      } else if (error.status >= 400 && error.status < 500) {
        setError("token", { message: t('login.errors.authError', { message: error.message }) });
      } else if (error.name === 'TypeError' || error.message.includes('fetch')) {
        setError("serverUrl", { message: t('login.errors.connectionFailed') });
      } else {
        setError("serverUrl", { message: t('login.errors.connectionError', { message: error.message }) });
      }
    }
  };

  return (
    <div className="min-h-screen bg-background flex items-center justify-center">
      <div className="w-full max-w-md p-4">
        <Card className="bg-glass-enhanced p-8 space-content animate-slide-up">
          <div className="flex items-start justify-between mb-6">
            <div className="space-y-3 flex-1">
              <div className="flex items-center space-x-3">
                <div className="w-10 h-10 bg-button-primary rounded-xl flex items-center justify-center hover-scale">
                  <SealboxIcon size="sm" className="text-white" />
                </div>
                <h1 className="heading-lg text-gradient">{t('login.title')}</h1>
              </div>
              <p className="body-sm text-muted-foreground text-balance">
                {t('login.subtitle')}
              </p>
            </div>
            <LanguageSelector />
          </div>

          <form onSubmit={handleSubmit(onSubmit)} className="space-content">
            <div className="space-tight">
              <Label htmlFor="serverUrl" className="text-sm font-medium">
                {t('login.serverUrl')}
              </Label>
              <Input
                id="serverUrl"
                type="url"
                placeholder={t('login.serverUrlPlaceholder')}
                className="h-11 bg-background/80 border-border/60 focus:bg-card focus:border-primary/40 transition-all duration-150"
                {...register("serverUrl")}
              />
              {errors.serverUrl && (
                <Alert variant="destructive" className="py-2 text-sm">
                  {errors.serverUrl.message}
                </Alert>
              )}
            </div>

            <div className="space-tight">
              <Label htmlFor="token" className="text-sm font-medium">
                {t('login.token')}
              </Label>
              <Input
                id="token"
                type="password"
                placeholder={t('login.tokenPlaceholder')}
                className="h-11 bg-background/80 border-border/60 focus:bg-card focus:border-primary/40 transition-all duration-150"
                {...register("token")}
              />
              {errors.token && (
                <Alert variant="destructive" className="py-2 text-sm">
                  {errors.token.message}
                </Alert>
              )}
            </div>

            <Button
              type="submit"
              size="lg"
              className="w-full bg-button-primary text-white button-gradient-hover font-medium"
              disabled={isSubmitting}
            >
              {isSubmitting ? (
                <div className="flex items-center space-x-2">
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  <span>{t('login.connecting')}</span>
                </div>
              ) : (
                t('login.connect')
              )}
            </Button>
          </form>

          <div className="p-4 bg-textured-muted rounded-lg border border-border/30 space-tight">
            <p className="text-sm font-medium text-foreground">{t('login.firstTime')}</p>
            <ol className="list-decimal list-inside space-minimal text-sm text-muted-foreground">
              <li>{t('login.steps.start')}</li>
              <li>{t('login.steps.setToken')}</li>
              <li>{t('login.steps.enterDetails')}</li>
            </ol>
          </div>
        </Card>
      </div>
    </div>
  );
}