// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 扩展源：Pack `contributions.search` 静态条目（宿主已聚合 packId/packName，
 * 此处过滤非法 / 未知路由）。匹配 翻译后标题 + 关键词；Enter 跳声明路由；
 * 副标题显示 Pack 名；无条目时源与 chip 一起隐藏。
 */

import type { Router } from "vue-router";
import {
  getSearchExtras,
  type FeaturePackSearchEntry,
} from "../../state/featurePack/contributions.ts";
import type {
  SearchResultItem,
  SearchSource,
  SearchSourceResult,
} from "../types.ts";

export type ExtensionsSourceDeps = {
  router: Router;
  /** labelKey 翻译（locale 变化经壳层 rerun 刷新）。 */
  t: (key: string) => string;
};

export function createExtensionsSource(
  deps: ExtensionsSourceDeps,
): SearchSource {
  return {
    id: "extensions",
    labelKey: "searchPalette.scopes.extensions",
    enabled: () => getSearchExtras().length > 0,
    unavailableKey: "searchPalette.status.extensionsUnavailable",
    search(query: string): SearchSourceResult {
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      const needle = q.toLowerCase();
      const items: SearchResultItem[] = [];
      for (const entry of getSearchExtras()) {
        const title = deps.t(entry.labelKey);
        const haystack = [title, ...entry.keywords]
          .join(" ")
          .toLowerCase();
        if (!haystack.includes(needle)) continue;
        items.push(extensionItem(entry, title, deps.router));
      }
      return { status: "ready", items };
    },
  };
}

function extensionItem(
  entry: FeaturePackSearchEntry,
  title: string,
  router: Router,
): SearchResultItem {
  return {
    key: `extensions:${entry.packId}:${entry.id}`,
    sourceId: "extensions",
    type: "extension",
    title,
    // 副标题显示 Pack 名。
    subtitle: entry.packName,
    activate: () => {
      void router.push(entry.route);
    },
  };
}
