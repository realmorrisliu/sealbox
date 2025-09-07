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
// Deprecated color helpers were removed in favor of semantic Badge/Alert variants.
