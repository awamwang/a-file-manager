// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 本机会话内 Navigator 目录 listing LRU（FIL-58）。
 * 切标签/回访同路径时先粉缓存，避免大目录重复 IPC + hydrate 卡顿。
 */

import { normalizePathKey } from "./pathKey.ts";

/** 热路径条数上限；超出按插入序淘汰最旧项。 */
export const LISTING_CACHE_MAX_ENTRIES = 12;
/** 缓存命中且未过期时可跳过后台 re-list（毫秒）。 */
export const LISTING_CACHE_FRESH_MS = 45_000;

type CacheEntry = {
  listing: unknown;
  cachedAt: number;
};

const cache = new Map<string, CacheEntry>();

export function listingCacheKey(path: string, showHidden: boolean): string {
  return `${normalizePathKey(path)}|h=${showHidden ? 1 : 0}`;
}

export function getCachedListing<T>(key: string): T | null {
  const hit = cache.get(key);
  if (!hit) return null;
  // LRU：再插入到末尾。
  cache.delete(key);
  cache.set(key, hit);
  return hit.listing as T;
}

export function isListingCacheFresh(key: string, now = Date.now()): boolean {
  const hit = cache.get(key);
  if (!hit) return false;
  return now - hit.cachedAt <= LISTING_CACHE_FRESH_MS;
}

export function putCachedListing(key: string, listing: unknown, now = Date.now()): void {
  if (cache.has(key)) cache.delete(key);
  cache.set(key, { listing, cachedAt: now });
  while (cache.size > LISTING_CACHE_MAX_ENTRIES) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
}

/** 删改/刷新后作废；不传 path 则清空。 */
export function invalidateListingCache(path?: string): void {
  if (path == null || !path.trim()) {
    cache.clear();
    return;
  }
  const needle = `${normalizePathKey(path)}|h=`;
  for (const key of [...cache.keys()]) {
    if (key.startsWith(needle)) {
      cache.delete(key);
    }
  }
}

export function clearListingCache(): void {
  cache.clear();
}

/** 测试用：当前条目数。 */
export function listingCacheSize(): number {
  return cache.size;
}
