import { createFileRoute, useRouter } from "@tanstack/react-router";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert } from "@/components/ui/alert";
import { LanguageSelector } from "@/components/i18n/language-selector";
import { ThemeToggle } from "@/components/theme/theme-toggle";
import { useAuthStore } from "@/stores/auth";
import { useConfigStore } from "@/stores/config";
import { createApiClient } from "@/lib/api";
import { useTranslation } from "react-i18next";

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

  // Use static English messages for validation schema to avoid hydration issues
  const loginSchema = z.object({
    serverUrl: z.string().url("Please enter a valid server URL"),
    token: z.string().min(1, "Please enter authentication token"),
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
        setError("token", { message: t("login.errors.authFailed") });
      } else if (error.status >= 400 && error.status < 500) {
        setError("token", {
          message: t("login.errors.authError", { message: error.message }),
        });
      } else if (
        error.name === "TypeError" ||
        error.message.includes("fetch")
      ) {
        setError("serverUrl", { message: t("login.errors.connectionFailed") });
      } else {
        setError("serverUrl", {
          message: t("login.errors.connectionError", { message: error.message }),
        });
      }
    }
  };

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <Card className="w-full max-w-md p-8">
        {/* Header */}
        <div className="flex items-start justify-between mb-6">
          <div className="flex-1">
            <div className="flex items-center space-x-3 mb-2">
              <div className="text-lg">🦭</div>
              <div className="flex items-center space-x-2">
                <h1 className="text-xl font-bold text-foreground">
                  {t("login.title")}
                </h1>
              </div>
            </div>
            <p className="text-sm text-muted-foreground">
              {t("login.subtitle")}
            </p>
          </div>
          <div className="flex items-center space-x-1">
            <ThemeToggle />
            <LanguageSelector />
          </div>
        </div>

        {/* Login Form */}
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="serverUrl" className="text-sm font-medium">
              {t("login.serverUrl")}
            </Label>
            <Input
              id="serverUrl"
              type="url"
              placeholder={t("login.serverUrlPlaceholder")}
              {...register("serverUrl")}
            />
            {errors.serverUrl && (
              <Alert variant="destructive" className="py-2 text-sm">
                {errors.serverUrl.message}
              </Alert>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="token" className="text-sm font-medium">
              {t("login.token")}
            </Label>
            <Input
              id="token"
              type="password"
              placeholder={t("login.tokenPlaceholder")}
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
            className="w-full"
            disabled={isSubmitting}
          >
            {isSubmitting ? (
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                <span>{t("login.connecting")}</span>
              </div>
            ) : (
              t("login.connect")
            )}
          </Button>
        </form>

        {/* Help Section */}
        <div className="mt-6 p-4 bg-muted/50 rounded-lg">
          <p className="text-sm font-medium text-foreground mb-2">
            {t("login.firstTime")}
          </p>
          <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
            <li>{t("login.steps.start")}</li>
            <li>{t("login.steps.setToken")}</li>
            <li>{t("login.steps.enterDetails")}</li>
          </ol>
        </div>
      </Card>
    </div>
  );
}
