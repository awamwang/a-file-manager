// SPDX-License-Identifier: GPL-3.0-or-later

import type { Router } from "vue-router";
import type { AppCommand } from "./types";

export type TranslateFn = (key: string) => string;

export type CommandPanelDeps = {
  router: Router;
  /** Ensure Navigator route then go up one folder. */
  ensureNavigatorThenGoUp: () => Promise<void>;
  refreshHealth: () => void | Promise<void>;
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

  return [
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
  ];
}
