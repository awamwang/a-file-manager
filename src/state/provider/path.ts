// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider 虚拟路径（`awam://…`）检测与父路径推导。 */

/** Provider 根目录（后端列表）。 */
export const PROVIDER_ROOT = "awam://root";

export type ProviderQueryParam = {
  key: string;
  label?: string | null;
  required?: boolean;
};

/** 是否为 Provider 方案路径（含根与各 backend）。 */
export function isProviderPath(path: string | null | undefined): boolean {
  if (!path) return false;
  return path.trim().toLowerCase().startsWith("awam://");
}

/** 拆出 path 与 `?` 起的 query 串。 */
function splitProviderQuery(path: string): { path: string; query: string } {
  const q = path.indexOf("?");
  if (q < 0) return { path, query: "" };
  return { path: path.slice(0, q), query: path.slice(q) };
}

function queryRecordFromString(query: string): Record<string, string> {
  if (!query) return {};
  const raw = query.startsWith("?") ? query.slice(1) : query;
  if (!raw) return {};
  const out: Record<string, string> = {};
  const params = new URLSearchParams(raw);
  params.forEach((value, key) => {
    out[key] = value;
  });
  return out;
}

function queryStringFromRecord(params: Record<string, string>): string {
  const keys = Object.keys(params).sort();
  if (!keys.length) return "";
  const search = new URLSearchParams();
  for (const key of keys) {
    search.set(key, params[key]!);
  }
  const serialized = search.toString();
  return serialized ? `?${serialized}` : "";
}

/** 规范化不含 query 的 provider 路径部分。 */
function normalizeProviderBase(path: string): string {
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
 * 规范导航用 URI：去尾斜杠；`awam` / backend 小写；**path 原样保留**；query 保留并键排序稳定。
 * browser `Profile%201` 等对大小写敏感，禁止整串 toLowerCase。
 * `awam://` / 空 backend → `awam://root`。
 */
export function toProviderUri(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) return "";
  if (!isProviderPath(trimmed)) return trimmed;

  const { path: pathPart, query } = splitProviderQuery(trimmed);
  const base = normalizeProviderBase(pathPart);
  if (!query) return base;
  const params = queryRecordFromString(query);
  return base + queryStringFromRecord(params);
}

export function getProviderQuery(path: string): Record<string, string> {
  const { query } = splitProviderQuery(path.trim());
  return queryRecordFromString(query);
}

/** backend 段（小写）；根为 `root`；非 provider 为 `null`。 */
export function providerBackendId(path: string): string | null {
  const trimmed = path.trim();
  if (!isProviderPath(trimmed)) return null;
  const { path: pathPart } = splitProviderQuery(trimmed);
  const normalized = normalizeProviderBase(pathPart);
  if (!normalized || normalized === PROVIDER_ROOT) {
    return normalized === PROVIDER_ROOT ? "root" : null;
  }
  const rest = normalized.slice("awam://".length);
  const slash = rest.indexOf("/");
  if (slash < 0) return rest || null;
  return rest.slice(0, slash) || null;
}

/** 顶层 `search` backend 在发现/导航中屏蔽。 */
export function isBlockedSearchProvider(path: string): boolean {
  return providerBackendId(path) === "search";
}

/** 合并或删除 query 键；`null`/空串删除该键。 */
export function withProviderQuery(
  path: string,
  patch: Record<string, string | null | undefined>,
): string {
  const trimmed = path.trim();
  const { path: pathPart, query } = splitProviderQuery(trimmed);
  const params = queryRecordFromString(query);
  for (const [key, value] of Object.entries(patch)) {
    if (value == null || value === "") {
      delete params[key];
    } else {
      params[key] = value;
    }
  }
  const base = isProviderPath(pathPart)
    ? normalizeProviderBase(pathPart)
    : pathPart;
  return base + queryStringFromRecord(params);
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
 * `awam://root` → `null`（由调用方落到 Guide）；
 * query 不入父级 URI。
 */
export function providerParentPath(path: string): string | null {
  const { path: pathPart } = splitProviderQuery(path.trim());
  const uri = normalizeProviderBase(pathPart);
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
  return normalizeProviderBase(`awam://${parentRest}`);
}

/** 标签/面包屑末段名（百分号解码后展示）；不含 query。 */
export function providerLeafName(path: string): string {
  const { path: pathPart } = splitProviderQuery(path.trim());
  const uri = normalizeProviderBase(pathPart);
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
