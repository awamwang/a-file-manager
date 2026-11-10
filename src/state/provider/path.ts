// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider 虚拟路径（`awam://…`）检测与父路径推导。 */

/** Provider 根目录（后端列表）。 */
export const PROVIDER_ROOT = "awam://root";

export function normalizeProviderKey(path: string): string {
  const trimmed = path.trim().toLowerCase();
  if (!trimmed.startsWith("awam://")) return trimmed;
  // 去掉 path 段尾斜杠，但保留方案前缀 `awam://`。
  const body = trimmed.slice("awam://".length).replace(/\/+$/, "");
  return body ? `awam://${body}` : "awam://root";
}

/** 是否为 Provider 方案路径（含根与各 backend）。 */
export function isProviderPath(path: string | null | undefined): boolean {
  if (!path) return false;
  return path.trim().toLowerCase().startsWith("awam://");
}

/** 规范展示/导航用 URI（去尾斜杠；空或非法仍返回原 trim）。 */
export function toProviderUri(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) return "";
  if (!isProviderPath(trimmed)) return trimmed;
  return normalizeProviderKey(trimmed);
}

/**
 * Provider 面包屑父级：
 * `awam://rclone/a/b` → `awam://rclone/a`；
 * `awam://rclone` → `awam://root`；
 * `awam://root` → `null`（由调用方落到 Guide）。
 */
export function providerParentPath(path: string): string | null {
  const uri = toProviderUri(path);
  if (!uri || !isProviderPath(uri)) return null;
  const key = normalizeProviderKey(uri);
  if (key === normalizeProviderKey(PROVIDER_ROOT) || key === "awam://") {
    return null;
  }
  const rest = uri.slice("awam://".length);
  const slash = rest.lastIndexOf("/");
  if (slash < 0) {
    return PROVIDER_ROOT;
  }
  const parentRest = rest.slice(0, slash);
  if (!parentRest) return PROVIDER_ROOT;
  return `awam://${parentRest}`;
}

/** 标签/面包屑末段名。 */
export function providerLeafName(path: string): string {
  const uri = toProviderUri(path);
  if (!uri) return "";
  if (normalizeProviderKey(uri) === normalizeProviderKey(PROVIDER_ROOT)) {
    return "root";
  }
  const rest = uri.slice("awam://".length);
  const parts = rest.split("/").filter(Boolean);
  return parts[parts.length - 1] || uri;
}
