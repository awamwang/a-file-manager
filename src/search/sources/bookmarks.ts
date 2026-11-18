// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 浏览器书签源：联邦检索 awam://browser 快照（标题 + URL + 文件夹路径）；
 * 副文案为浏览器/配置名；Enter → resolve open_url；离线有快照可只读并标过期。
 */

import { openProviderEntry } from "../../state/provider/browse.ts";
import {
  loadBookmarkSnapshot,
  refreshBookmarkSnapshot,
  type BookmarkSnapshotEntry,
} from "../../state/bookmarkSnapshot.ts";
import type {
  SearchActivateOutcome,
  SearchQueryOptions,
  SearchResultItem,
  SearchSource,
  SearchSourceResult,
} from "../types.ts";
import { matchText } from "../match.ts";

/** 副文案：浏览器 / 配置名。 */
export function bookmarkSubtitle(entry: BookmarkSnapshotEntry): string {
  const parts = [entry.browserLabel, entry.profileLabel].filter((part) =>
    Boolean(part.trim()),
  );
  return parts.join(" / ");
}

/** 匹配标题、URL、文件夹路径（模糊=子序列）。 */
export function matchBookmarkEntry(
  entry: BookmarkSnapshotEntry,
  query: string,
  fuzzy: boolean,
): boolean {
  if (matchText(entry.title, query, fuzzy)) return true;
  if (matchText(entry.url, query, fuzzy)) return true;
  if (entry.folderPath && matchText(entry.folderPath, query, fuzzy)) return true;
  return false;
}

function bookmarkItem(entry: BookmarkSnapshotEntry): SearchResultItem {
  return {
    key: `bookmarks:${entry.uri}`,
    sourceId: "bookmarks",
    type: "bookmark",
    title: entry.title,
    subtitle: bookmarkSubtitle(entry),
    icon: "bookmark",
    activate: async (): Promise<SearchActivateOutcome> => {
      try {
        await openProviderEntry(entry.uri);
      } catch {
        return { toastKey: "searchPalette.openFailed", kind: "error" };
      }
    },
  };
}

export function createBookmarksSource(): SearchSource {
  return {
    id: "bookmarks",
    labelKey: "searchPalette.scopes.bookmarks",
    enabled: () => true,
    unavailableKey: "searchPalette.status.bookmarksUnavailable",
    unavailableActionLabelKey: "searchPalette.status.refreshBookmarks",
    retainPanelOnStatusAction: true,
    unavailableAction: async () => {
      await refreshBookmarkSnapshot();
    },
    async search(
      query: string,
      options?: SearchQueryOptions,
    ): Promise<SearchSourceResult> {
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      const fuzzy = options?.fuzzy ?? false;
      const loaded = await loadBookmarkSnapshot();
      if (loaded.unavailable || !loaded.snapshot) {
        return { status: "unavailable", items: [] };
      }
      const items: SearchResultItem[] = [];
      for (const entry of loaded.snapshot.entries) {
        if (matchBookmarkEntry(entry, q, fuzzy)) {
          items.push(bookmarkItem(entry));
        }
      }
      return {
        status: "ready",
        items,
        hintKey: loaded.stale
          ? "searchPalette.status.bookmarksStale"
          : undefined,
      };
    },
  };
}
