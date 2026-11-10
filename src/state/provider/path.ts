// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider 虚拟路径（`awam://…`）检测与父路径推导。 */

/** Provider 根目录（后端列表）。 */
export const PROVIDER_ROOT = "awam://root";

/** 是否为 Provider 方案路径（含根与各 backend）。 */
export function isProviderPath(path: string | null | undefined): boolean {
  if (!path) return false;
  return path.trim().toLowerCase().startsWith("awam://");
}

/**
 * 规范导航用 URI：去尾斜杠；`awam` / backend 小写；**path 原样保留**。
 * browser `Profile%201` 等对大小写敏感，禁止整串 toLowerCase。
 * `awam://` / 空 backend → `awam://root`。
 */
export function toProviderUri(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) return "";
  if (!isProviderPath(trimmed)) return trimmed;

  const body = trimmed.slice("awam://".length).replace(/\/+$/, "");
  if (!body) return PROVIDER_ROOT;

  const slash = body.indexOf("/");
  if (slash < 0) {
    const backend = body.toLowerCase();
    return !backend || backend === "root" ? PROVIDER_ROOT : `awam://${backend}`;
  }

  const backend = body.slice(0, slash).toLowerCase() || "root";
  const rest = body.slice(slash + 1).replace(/\/+$/, "");
  if (!rest) {
    return !backend || backend === "root" ? PROVIDER_ROOT : `awam://${backend}`;
  }
  return `awam://${backend}/${rest}`;
}

/**
 * 比较键：与 `toProviderUri` 相同（path 大小写敏感）。
 * 非 provider 路径回退为小写 trim。
 */
export function normalizeProviderKey(path: string): string {
  if (!isProviderPath(path)) return path.trim().toLowerCase();
  return toProviderUri(path);
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
  if (uri === PROVIDER_ROOT) {
    return null;
  }
  const rest = uri.slice("awam://".length);
  const slash = rest.lastIndexOf("/");
  if (slash < 0) {
    return PROVIDER_ROOT;
  }
  const parentRest = rest.slice(0, slash);
  if (!parentRest) return PROVIDER_ROOT;
  return toProviderUri(`awam://${parentRest}`);
}

/** 标签/面包屑末段名（百分号解码后展示）。 */
export function providerLeafName(path: string): string {
  const uri = toProviderUri(path);
  if (!uri) return "";
  if (uri === PROVIDER_ROOT) {
    return "root";
  }
  const rest = uri.slice("awam://".length);
  const parts = rest.split("/").filter(Boolean);
  const leaf = parts[parts.length - 1] || uri;
  try {
    return decodeURIComponent(leaf);
  } catch {
    return leaf;
  }
}
