import type { SecretInfo, SecretUIData, SecretStatus } from "./types";

/**
 * 将 API 返回的 SecretInfo 转换为 UI 展示数据
 */
export function convertSecretToUIData(secret: SecretInfo): SecretUIData {
  const now = Date.now() / 1000;
  const isExpired = secret.expires_at && now > secret.expires_at;
  const isExpiring =
    secret.expires_at && !isExpired && secret.expires_at - now < 7 * 24 * 3600; // Expiring in 7 days

  return {
    // Original API fields
    key: secret.key,
    version: secret.version,
    created_at: secret.created_at,
    updated_at: secret.updated_at,
    expires_at: secret.expires_at,
    // Computed display fields
    status: isExpired ? "expired" : isExpiring ? "expiring" : "active",
    createdAt: new Date(secret.created_at * 1000).toISOString().split("T")[0],
    updatedAt: new Date(secret.updated_at * 1000).toISOString().split("T")[0],
    expiresAt: secret.expires_at
      ? new Date(secret.expires_at * 1000).toISOString().split("T")[0]
      : undefined,
  };
}

/**
 * 计算密钥统计信息
 */
export function calculateSecretStats(secrets: SecretUIData[]) {
  return {
    total: secrets.length,
    expiring: secrets.filter((s) => s.status === "expiring").length,
    expired: secrets.filter((s) => s.status === "expired").length,
  };
}

/**
 * 根据搜索条件过滤密钥
 */
export function filterSecrets(secrets: SecretUIData[], searchTerm: string) {
  if (!searchTerm.trim()) return secrets;

  return secrets.filter((secret) =>
    secret.key.toLowerCase().includes(searchTerm.toLowerCase()),
  );
}

/**
 * 获取状态对应的颜色类名
 */
export function getStatusColor(status: SecretStatus): string {
  switch (status) {
    case "active":
      return "text-green-600";
    case "expiring":
      return "text-yellow-600";
    case "expired":
      return "text-red-600";
    default:
      return "text-gray-600";
  }
}

/**
 * 获取状态对应的 Badge 样式类名
 */
export function getStatusBadge(status: SecretStatus): string {
  switch (status) {
    case "active":
      return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200";
    case "expiring":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200";
    case "expired":
      return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200";
  }
}
