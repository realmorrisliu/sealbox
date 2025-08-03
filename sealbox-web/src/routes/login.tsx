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
import { LanguageSelector } from "@/components/ui/language-selector";
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

  // 如果已经认证，跳转到首页
  useEffect(() => {
    if (isAuthenticated) {
      router.navigate({ to: "/" });
    }
  }, [isAuthenticated, router]);

  const onSubmit = async (data: LoginForm) => {
    try {
      // 测试连接 - 使用 readiness 端点来验证服务器状态和认证
      const api = createApiClient(data.serverUrl, data.token);
      await api.readiness();
      
      // 连接成功，保存认证信息
      login(data.token, data.serverUrl);
      setDefaultServerUrl(data.serverUrl);
      
      // 跳转到首页
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
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md p-6 space-y-6">
        <div className="flex items-center justify-between">
          <div className="text-center space-y-2 flex-1">
            <h1 className="text-2xl font-bold">{t('login.title')}</h1>
            <p className="text-muted-foreground">
              {t('login.subtitle')}
            </p>
          </div>
          <LanguageSelector />
        </div>

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="serverUrl">{t('login.serverUrl')}</Label>
            <Input
              id="serverUrl"
              type="url"
              placeholder={t('login.serverUrlPlaceholder')}
              {...register("serverUrl")}
            />
            {errors.serverUrl && (
              <Alert variant="destructive" className="py-2">
                {errors.serverUrl.message}
              </Alert>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="token">{t('login.token')}</Label>
            <Input
              id="token"
              type="password"
              placeholder={t('login.tokenPlaceholder')}
              {...register("token")}
            />
            {errors.token && (
              <Alert variant="destructive" className="py-2">
                {errors.token.message}
              </Alert>
            )}
          </div>

          <Button
            type="submit"
            className="w-full"
            disabled={isSubmitting}
          >
            {isSubmitting ? t('login.connecting') : t('login.connect')}
          </Button>
        </form>

        <div className="text-sm text-muted-foreground space-y-1">
          <p>{t('login.firstTime')}</p>
          <ol className="list-decimal list-inside space-y-1 text-xs">
            <li>{t('login.steps.start')}</li>
            <li>{t('login.steps.setToken')}</li>
            <li>{t('login.steps.enterDetails')}</li>
          </ol>
        </div>
      </Card>
    </div>
  );
}