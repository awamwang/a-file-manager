// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 浏览器书签扁平快照：按需递归 list(awam://browser/…) 建缓存，供搜索面板书签源过滤。
 * TTL 10 分钟；失败不覆盖已有成功快照；网关离线时有快照可只读检索并标 stale。
 */

import { invoke } from "@tauri-apps/api/core";
import {
  shouldThrowProviderListError,
  type ProviderDirListingWire,
  type ProviderEntryWire,
} from "./provider/listing.ts";
import type { ProviderGatewayStatus } from "./providerCache.ts";

/** 单条书签（扁平）：标题 / URL / 文件夹路径 / 浏览器·配置显示名。 */
export type BookmarkSnapshotEntry = {
  uri: string;
  title: string;
  url: string;
  /** 根收藏夹起的文件夹路径（`/` 分隔），不含浏览器与配置名。 */
  folderPath: string;
  browserLabel: string;
  profileLabel: string;
};

export type BookmarkSnapshot = {
  entries: readonly BookmarkSnapshotEntry[];
  fetchedAt: number;
};

export type BookmarkSnapshotLoad = {
  snapshot: BookmarkSnapshot | null;
  /** 无可用快照（网关离线且从未成功拉取，或爬树失败且无旧快照）。 */
  unavailable: boolean;
  /** 正在使用过期或离线只读快照。 */
  stale: boolean;
};

const BROWSER_ROOT = "awam://browser";
/** 快照 TTL：取规格 5–15 分钟中位。 */
export const BOOKMARK_SNAPSHOT_TTL_MS = 10 * 60_000;
const LIST_CONCURRENCY = 4;
const MAX_DIR_VISITS = 2_000;
const MAX_BOOKMARKS = 10_000;

type CrawlContext = {
  browserLabel: string;
  profileLabel: string;
  /** 自收藏夹根起的文件夹名。 */
  folderSegments: string[];
  /** 0=产品，1=配置，≥2=收藏夹树。 */
  depth: number;
};

type QueueItem = { uri: string; ctx: CrawlContext };

let cached: BookmarkSnapshot | null = null;
let inflight: Promise<BookmarkSnapshotLoad> | null = null;

/** 清空缓存（测试 / 手动刷新）。 */
export function resetBookmarkSnapshotCache(): void {
  cached = null;
}

/** 探测网关是否在线。 */
async function isGatewayOnline(): Promise<boolean> {
  try {
    const status = await invoke<ProviderGatewayStatus>("provider_gateway_status");
    return Boolean(status.online);
  } catch {
    return false;
  }
}

function entryUrl(extra: Record<string, unknown> | undefined): string | null {
  if (!extra) return null;
  const url = extra.url;
  return typeof url === "string" && url.trim() ? url.trim() : null;
}

function isBookmarkLeaf(entry: ProviderEntryWire): boolean {
  if (entry.isDir) return false;
  const extra = entry.extra;
  if (!extra) return false;
  if (typeof extra.kind === "string" && extra.kind.trim().toLowerCase() === "bookmark") {
    return Boolean(entryUrl(extra));
  }
  return Boolean(entryUrl(extra));
}

/** 默认经 Tauri `provider_list` 列举；测试可注入假列表。 */
export type BookmarkListFn = (uri: string) => Promise<ProviderDirListingWire>;

async function defaultListDir(uri: string): Promise<ProviderDirListingWire> {
  const listing = await invoke<ProviderDirListingWire>("provider_list", { uri });
  if (shouldThrowProviderListError(listing)) {
    throw new Error(listing.error ?? "provider_list failed");
  }
  return listing;
}

/**
 * 广度优先爬取 `awam://browser`，产出扁平书签列表。
 * 并发受限；目录访问与书签条数有软上限，避免异常树拖垮面板。
 */
export async function crawlBrowserBookmarks(
  listDir: BookmarkListFn = defaultListDir,
): Promise<BookmarkSnapshotEntry[]> {
  const entries: BookmarkSnapshotEntry[] = [];
  const seenDirs = new Set<string>();
  const queue: QueueItem[] = [
    {
      uri: BROWSER_ROOT,
      ctx: {
        browserLabel: "",
        profileLabel: "",
        folderSegments: [],
        depth: -1,
      },
    },
  ];
  let dirVisits = 0;
  let cursor = 0;

  const processOne = async (item: QueueItem): Promise<void> => {
    if (dirVisits >= MAX_DIR_VISITS || entries.length >= MAX_BOOKMARKS) return;
    const key = item.uri.toLowerCase();
    if (seenDirs.has(key)) return;
    seenDirs.add(key);
    dirVisits += 1;

    let listing: ProviderDirListingWire;
    try {
      listing = await listDir(item.uri);
    } catch {
      return;
    }

    for (const entry of listing.entries) {
      if (entries.length >= MAX_BOOKMARKS) break;
      if (isBookmarkLeaf(entry)) {
        const url = entryUrl(entry.extra) ?? "";
        entries.push({
          uri: entry.uri,
          title: entry.name,
          url,
          folderPath: item.ctx.folderSegments.join("/"),
          browserLabel: item.ctx.browserLabel,
          profileLabel: item.ctx.profileLabel,
        });
        continue;
      }
      if (!entry.isDir) continue;

      const nextDepth = item.ctx.depth + 1;
      let browserLabel = item.ctx.browserLabel;
      let profileLabel = item.ctx.profileLabel;
      let folderSegments = item.ctx.folderSegments;
      if (nextDepth === 0) {
        // 产品层：Microsoft Edge / Google Chrome
        browserLabel = entry.name;
      } else if (nextDepth === 1) {
        // 配置层：显示名（如「日常」）
        profileLabel = entry.name;
      } else {
        // 收藏夹根与嵌套文件夹
        folderSegments = [...folderSegments, entry.name];
      }
      queue.push({
        uri: entry.uri,
        ctx: {
          browserLabel,
          profileLabel,
          folderSegments,
          depth: nextDepth,
        },
      });
    }
  };

  const worker = async (): Promise<void> => {
    while (cursor < queue.length) {
      if (dirVisits >= MAX_DIR_VISITS || entries.length >= MAX_BOOKMARKS) return;
      const index = cursor;
      cursor += 1;
      const item = queue[index];
      if (!item) return;
      await processOne(item);
    }
  };

  // 队列会在 worker 运行中增长；反复补工人直到排空。
  while (cursor < queue.length) {
    if (dirVisits >= MAX_DIR_VISITS || entries.length >= MAX_BOOKMARKS) break;
    const pending = queue.length - cursor;
    const workerCount = Math.min(LIST_CONCURRENCY, pending);
    await Promise.all(Array.from({ length: workerCount }, () => worker()));
  }

  return entries;
}

async function buildLoad(force: boolean): Promise<BookmarkSnapshotLoad> {
  const online = await isGatewayOnline();
  const ttlOk =
    cached !== null && Date.now() - cached.fetchedAt < BOOKMARK_SNAPSHOT_TTL_MS;

  if (!force && ttlOk) {
    return { snapshot: cached, unavailable: false, stale: false };
  }

  if (!online) {
    if (cached) {
      return { snapshot: cached, unavailable: false, stale: true };
    }
    return { snapshot: null, unavailable: true, stale: false };
  }

  try {
    const entries = await crawlBrowserBookmarks(defaultListDir);
    cached = { entries, fetchedAt: Date.now() };
    return { snapshot: cached, unavailable: false, stale: false };
  } catch {
    if (cached) {
      return { snapshot: cached, unavailable: false, stale: true };
    }
    return { snapshot: null, unavailable: true, stale: false };
  }
}

/**
 * 读取书签快照。成功结果按 TTL 缓存；失败不抹掉旧快照。
 * `force` 时忽略 TTL 重新爬树（手动刷新）。
 */
export function loadBookmarkSnapshot(options?: {
  force?: boolean;
}): Promise<BookmarkSnapshotLoad> {
  const force = options?.force === true;
  if (!force && cached && Date.now() - cached.fetchedAt < BOOKMARK_SNAPSHOT_TTL_MS) {
    return Promise.resolve({
      snapshot: cached,
      unavailable: false,
      stale: false,
    });
  }
  if (!inflight) {
    const pending = buildLoad(force);
    inflight = pending;
    void pending.finally(() => {
      if (inflight === pending) inflight = null;
    });
  }
  return inflight;
}

/** 强制刷新快照（忽略 TTL）；供状态行「刷新」使用。 */
export function refreshBookmarkSnapshot(): Promise<BookmarkSnapshotLoad> {
  inflight = null;
  return loadBookmarkSnapshot({ force: true });
}
