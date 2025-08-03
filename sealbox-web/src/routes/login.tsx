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
import { useAuthStore } from "@/stores/auth";
import { useConfigStore } from "@/stores/config";
import { createApiClient } from "@/lib/api";

const loginSchema = z.object({
  serverUrl: z.string().url("Please enter a valid server URL"),
  token: z.string().min(1, "Please enter authentication token"),
});

type LoginForm = z.infer<typeof loginSchema>;

export const Route = createFileRoute("/login")({
  component: LoginPage,
});

function LoginPage() {
  const router = useRouter();
  const { login, isAuthenticated } = useAuthStore();
  const { defaultServerUrl, setDefaultServerUrl } = useConfigStore();
  
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
        setError("token", { message: "Authentication failed, please check your token" });
      } else if (error.status >= 400 && error.status < 500) {
        setError("token", { message: `Authentication error: ${error.message}` });
      } else if (error.name === 'TypeError' || error.message.includes('fetch')) {
        setError("serverUrl", { message: "Unable to connect to server, please check the URL" });
      } else {
        setError("serverUrl", { message: `Connection error: ${error.message}` });
      }
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md p-6 space-y-6">
        <div className="text-center space-y-2">
          <h1 className="text-2xl font-bold">Sealbox</h1>
          <p className="text-muted-foreground">
            Connect to your secret management server
          </p>
        </div>

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="serverUrl">Server URL</Label>
            <Input
              id="serverUrl"
              type="url"
              placeholder="http://localhost:8080"
              {...register("serverUrl")}
            />
            {errors.serverUrl && (
              <Alert variant="destructive" className="py-2">
                {errors.serverUrl.message}
              </Alert>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="token">Authentication Token</Label>
            <Input
              id="token"
              type="password"
              placeholder="Enter your Bearer Token"
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
            {isSubmitting ? "Connecting..." : "Connect"}
          </Button>
        </form>

        <div className="text-sm text-muted-foreground space-y-1">
          <p>First time using?</p>
          <ol className="list-decimal list-inside space-y-1 text-xs">
            <li>Start sealbox-server</li>
            <li>Set AUTH_TOKEN environment variable</li>
            <li>Enter server URL and token above</li>
          </ol>
        </div>
      </Card>
    </div>
  );
}