// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 文件源：`discovery_search` limit 8，Everything 自身规则匹配（面板不二次过滤）；
 * 勾选「模糊搜索」时改发 `*a*b*c*` 通配符子序列模式（limit 32）并按名称 fuzzyScore 重排取前 8。
 * - 180ms 防抖 + 序列号丢弃过期结果；
 * - 目录 Enter：openOrFocusNavigatorTab + 跳 /navigator；
 *   文件 Enter：openLocalEntry（遵循 core.fileDoubleClickAction，跳过 Provider 分支）；
 *   Ctrl+Enter：revealItemInDir（资源管理器定位）；
 * - 离线（Everything 未运行且备用搜索关闭）出状态行 + 打开搜索页。
 */

import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { Router } from "vue-router";
import {
  openLocalEntry,
  openOrFocusNavigatorTab,
  type NavigatorEntry,
} from "../../state/navigatorBrowse.ts";
import { requestShellIcons } from "../../state/shellIcons.ts";
import { matchByName } from "../match.ts";
import type {
  SearchActivateOutcome,
  SearchQueryOptions,
  SearchResultItem,
  SearchSource,
  SearchSourceResult,
} from "../types.ts";

type DiscoveryHit = {
  name: string;
  path: string;
  isDir: boolean;
};

type DiscoverySearchResult = {
  query: string;
  engine: "everything" | "fallback" | "unavailable";
  totalCount: number;
  offset: number;
  hits: DiscoveryHit[];
};

/** 输入防抖：面板每键触发，避免每个字符打一次 Everything IPC。 */
const DEBOUNCE_MS = 180;
/** 面板内文件结果上限（discovery_search limit，同时也是组上限）。 */
const FILE_LIMIT = 8;
/** 模糊模式一次拉取的候选数（客户端按名称 fuzzyScore 重排后取前 8）。 */
const FUZZY_FETCH_LIMIT = 32;

/**
 * 模糊模式的 Everything 查询：把查询转成子序列通配符模式 `*a*b*c*`
 * （仅字母 / 数字 / CJK 参与，其余字符剔除）；无有效字符时原样返回。
 */
export function fuzzyEverythingPattern(query: string): string {
  const chars = [...query].filter((ch) => /[\p{L}\p{N}]/u.test(ch));
  if (chars.length === 0) return query;
  return `*${chars.join("*")}*`;
}

let epoch = 0;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function normalizeHitPath(path: string): string {
  return path.replace(/\//g, "\\");
}

export function createFilesSource(router: Router): SearchSource {
  return {
    id: "files",
    labelKey: "searchPalette.scopes.files",
    enabled: () => true,
    unavailableKey: "searchPalette.status.filesUnavailable",
    unavailableActionLabelKey: "searchPalette.status.openDiscovery",
    unavailableAction: () => {
      void router.push("/discovery");
    },
    viewAll: {
      labelKey: "searchPalette.viewAllFiles",
      action: (query: string) => {
        const q = query.trim();
        void router.push({
          name: "discovery",
          query: q ? { q } : {},
        });
      },
    },
    async search(
      query: string,
      options?: SearchQueryOptions,
    ): Promise<SearchSourceResult> {
      const seq = ++epoch;
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      const fuzzy = options?.fuzzy ?? false;
      // 防抖：过期查询不再发起 IPC。
      await sleep(DEBOUNCE_MS);
      if (seq !== epoch) return { status: "ready", items: [] };
      let result: DiscoverySearchResult;
      try {
        result = await invoke<DiscoverySearchResult>("discovery_search", {
          // 模糊：Everything 通配符子序列模式 + 多拉候选，客户端按名称重排。
          query: fuzzy ? fuzzyEverythingPattern(q) : q,
          options: {
            offset: 0,
            limit: fuzzy ? FUZZY_FETCH_LIMIT : FILE_LIMIT,
          },
        });
      } catch {
        // Everything 离线且备用搜索关闭 / IPC 失败：出状态行。
        return { status: "unavailable", items: [] };
      }
      if (seq !== epoch) return { status: "ready", items: [] };
      const hits = fuzzy
        ? matchByName(result.hits, (hit) => hit.name, q)
            .slice(0, FILE_LIMIT)
            .map((match) => match.entry)
        : result.hits.slice(0, FILE_LIMIT);
      requestShellIcons(hits.map((hit) => normalizeHitPath(hit.path)));
      const items = hits.map((hit): SearchResultItem => {
        const path = normalizeHitPath(hit.path);
        const kind = hit.isDir ? "dir" : "file";
        return {
          key: `files:${path}:${hit.name}`,
          sourceId: "files",
          type: hit.isDir ? "folder" : "file",
          title: hit.name,
          subtitle: path,
          iconPath: path,
          activate: async ({ ctrl }): Promise<SearchActivateOutcome> => {
            if (ctrl) {
              void revealItemInDir(path);
              return;
            }
            if (hit.isDir) {
              void openOrFocusNavigatorTab(path);
              void router.push("/navigator");
              return;
            }
            const entry: NavigatorEntry = {
              name: hit.name,
              path,
              kind,
              size: 0,
            };
            try {
              await openLocalEntry(entry);
            } catch {
              // 打开失败（无关联程序 / 路径失效等）：面板转 toast。
              return { toastKey: "searchPalette.openFailed", kind: "error" };
            }
          },
        };
      });
      return { status: "ready", items };
    },
  };
}
