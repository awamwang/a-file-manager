// SPDX-License-Identifier: GPL-3.0-or-later
/** 项目源：仅匹配 displayName，Enter 跳 `/projects?slug=` 仅聚焦、不激活。 */

import { invoke } from "@tauri-apps/api/core";
import type { Router } from "vue-router";
import { matchByName } from "../match.ts";
import type {
  SearchSource,
  SearchSourceResult,
  SearchResultItem,
} from "../types.ts";

type WorkspaceListResult = {
  workspaces: Array<{
    path: string;
    project: { slug: string; displayName: string };
  }>;
};

/** 工作区列表 10s 缓存：面板连续输入时不反复读盘。 */
const LIST_TTL_MS = 10_000;
let cache: { at: number; promise: Promise<WorkspaceListResult> } | null = null;

function listWorkspaces(): Promise<WorkspaceListResult> {
  const now = Date.now();
  if (cache && now - cache.at < LIST_TTL_MS) return cache.promise;
  const promise = invoke<WorkspaceListResult>("projects_workspace_list");
  cache = { at: now, promise };
  promise.catch(() => {
    if (cache?.promise === promise) cache = null;
  });
  return promise;
}

export function createProjectsSource(router: Router): SearchSource {
  return {
    id: "projects",
    labelKey: "searchPalette.scopes.projects",
    enabled: () => true,
    unavailableKey: "searchPalette.status.projectsUnavailable",
    unavailableActionLabelKey: "searchPalette.status.openProjects",
    unavailableAction: () => {
      void router.push("/projects");
    },
    async search(query: string): Promise<SearchSourceResult> {
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      let list: WorkspaceListResult;
      try {
        list = await listWorkspaces();
      } catch {
        return { status: "unavailable", items: [] };
      }
      const items: SearchResultItem[] = matchByName(
        list.workspaces,
        (workspace) => workspace.project.displayName,
        q,
      ).map(({ entry }) => ({
        key: `projects:${entry.project.slug}`,
        sourceId: "projects",
        type: "project",
        title: entry.project.displayName,
        subtitle: entry.path,
        activate: () => {
          void router.push({
            name: "projects",
            query: { slug: entry.project.slug },
          });
        },
      }));
      return { status: "ready", items };
    },
  };
}
