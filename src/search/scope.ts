// SPDX-License-Identifier: GPL-3.0-or-later
/** 范围组合（Scope Set）：localStorage 持久化 + 归一化 + 切换。 */

import { SEARCH_SOURCE_ORDER, type SearchSourceId } from "./types.ts";

export const SEARCH_PALETTE_SCOPES_STORAGE_KEY = "afm.searchPaletteScopes";

export function isSearchSourceId(value: unknown): value is SearchSourceId {
  return (
    typeof value === "string" &&
    (SEARCH_SOURCE_ORDER as readonly string[]).includes(value)
  );
}

/**
 * 归一化持久化数据：非数组 / 空集 / 含未知 id 时返回 null（按「全部」处理）。
 * 「全部」= 全选，不是独立模式；含暂不可用源时保留选择，不自动回退。
 */
export function normalizePersistedScopes(raw: unknown): SearchSourceId[] | null {
  if (!Array.isArray(raw) || raw.length === 0) return null;
  if (!raw.every(isSearchSourceId)) return null;
  return [...new Set<SearchSourceId>(raw)];
}

export function loadScopes(storage: Storage | undefined): SearchSourceId[] | null {
  if (!storage) return null;
  try {
    return normalizePersistedScopes(
      JSON.parse(storage.getItem(SEARCH_PALETTE_SCOPES_STORAGE_KEY) ?? "null"),
    );
  } catch {
    return null;
  }
}

export function persistScopes(
  scopes: readonly SearchSourceId[],
  storage: Storage | undefined,
): void {
  if (!storage) return;
  try {
    storage.setItem(
      SEARCH_PALETTE_SCOPES_STORAGE_KEY,
      JSON.stringify([...new Set<SearchSourceId>(scopes)]),
    );
  } catch {
    /* 持久化失败不阻断面板。 */
  }
}

/**
 * 切换一个搜索源；关掉最后一个源回落「全部」。
 * `current` 是当前生效范围，`available` 是全部可用源。
 */
export function toggleScope(
  current: readonly SearchSourceId[],
  available: readonly SearchSourceId[],
  id: SearchSourceId,
): SearchSourceId[] {
  const next = current.includes(id)
    ? current.filter((scope) => scope !== id)
    : [...current, id];
  return next.length ? next : [...available];
}
