// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 文件源：`discovery_search` limit 8，Everything 自身规则匹配（面板不二次过滤）。
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
import type {
  SearchActivateOutcome,
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
    async search(query: string): Promise<SearchSourceResult> {
      const seq = ++epoch;
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      // 防抖：过期查询不再发起 IPC。
      await sleep(DEBOUNCE_MS);
      if (seq !== epoch) return { status: "ready", items: [] };
      let result: DiscoverySearchResult;
      try {
        result = await invoke<DiscoverySearchResult>("discovery_search", {
          query: q,
          options: { offset: 0, limit: FILE_LIMIT },
        });
      } catch {
        // Everything 离线且备用搜索关闭 / IPC 失败：出状态行。
        return { status: "unavailable", items: [] };
      }
      if (seq !== epoch) return { status: "ready", items: [] };
      const hits = result.hits.slice(0, FILE_LIMIT);
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
