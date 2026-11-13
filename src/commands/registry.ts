// SPDX-License-Identifier: GPL-3.0-or-later

import type { Router } from "vue-router";
import { AI_ADDON_ENABLED } from "../config/flavor.ts";
import { getCommandExtras } from "../state/featurePack/contributions.ts";
import type { AppCommand } from "./types.ts";

export type TranslateFn = (key: string) => string;

export type CommandPanelDeps = {
  router: Router;
  /** Ensure Navigator route then go up one folder. */
  ensureNavigatorThenGoUp: () => Promise<void>;
  refreshHealth: () => void | Promise<void>;
  /** Soft Reload Window (AIA-281) — remount UI, keep persisted data. */
  reloadWindow: () => void | Promise<void>;
  /** Localize command titles (vue-i18n `t`). */
  t: TranslateFn;
};

/**
 * Built-in commands wired to real shell routes / navigator / health.
 * Group ids stay English for stable ordering; titles are localized.
 */
export function buildAppCommands(deps: CommandPanelDeps): AppCommand[] {
  const go = (path: string) => {
    void deps.router.push(path);
  };
  const { t } = deps;

  const commands: AppCommand[] = [
    {
      id: "nav.go-up",
      title: t("commands.goUp"),
      group: "Navigate",
      keywords: ["parent", "up", "navigator", "explorer", "向上"],
      run: () => deps.ensureNavigatorThenGoUp(),
    },
    {
      id: "nav.open-navigator",
      title: t("commands.openNavigator"),
      group: "Navigate",
      keywords: ["browse", "files", "home", "explorer", "导航", "资源管理器", "导航器"],
      run: () => go("/navigator"),
    },
    {
      id: "view.discovery",
      title: t("commands.openDiscovery"),
      group: "View",
      keywords: ["everything", "search", "搜索", "发现"],
      run: () => go("/discovery"),
    },
    {
      id: "view.providers",
      title: t("commands.openProviders"),
      group: "View",
      keywords: ["rclone", "knowledge", "awam"],
      run: () => go("/providers"),
    },
    {
      id: "view.work-groups",
      title: t("commands.openWorkGroups"),
      group: "View",
      keywords: ["wg", "tabs", "工作组"],
      run: () => go("/work-groups"),
    },
    {
      id: "view.projects",
      title: t("commands.openProjects"),
      group: "View",
      keywords: ["project", "board", "项目"],
      run: () => go("/projects"),
    },
    {
      id: "view.ai",
      title: t("commands.openAi"),
      group: "View",
      keywords: ["ai", "mcp", "skills", "agent", "llm", "人工智能", "智能"],
      run: () => go("/ai"),
    },
    {
      id: "view.extensions",
      title: t("commands.openExtensions"),
      group: "View",
      keywords: ["marketplace", "vnext", "plugins", "扩展"],
      run: () => go("/extensions"),
    },
    {
      id: "view.settings",
      title: t("commands.openSettings"),
      group: "View",
      keywords: ["preferences", "core", "options", "设置"],
      run: () => go("/settings"),
    },
    {
      id: "view.health",
      title: t("commands.openHealth"),
      group: "View",
      keywords: ["status", "version", "diagnostics", "settings", "健康", "设置"],
      run: () => go("/settings/health"),
    },
    {
      id: "action.refresh-health",
      title: t("commands.refreshHealth"),
      group: "Actions",
      keywords: ["health", "reload", "status"],
      run: () => deps.refreshHealth(),
    },
    {
      id: "action.reload-window",
      title: t("commands.reloadWindow"),
      group: "Actions",
      keywords: [
        "reload",
        "window",
        "refresh",
        "vscode",
        "重载",
        "重新加载",
        "刷新窗口",
      ],
      run: () => deps.reloadWindow(),
    },
  ];

  const built = AI_ADDON_ENABLED
    ? commands
    : commands.filter((cmd) => cmd.id !== "view.ai");
  built.push(...getCommandExtras({ router: deps.router, t: deps.t }));
  return built;
}
