// SPDX-License-Identifier: GPL-3.0-or-later

import type { Router } from "vue-router";
import type { AppCommand } from "./types";

export type CommandPanelDeps = {
  router: Router;
  /** Ensure Navigator route then go up one folder. */
  ensureNavigatorThenGoUp: () => Promise<void>;
  refreshHealth: () => void | Promise<void>;
};

/**
 * Built-in commands wired to real shell routes / navigator / health.
 * Extend here as new app actions land — keep Vue thin; no Rust unless needed.
 */
export function buildAppCommands(deps: CommandPanelDeps): AppCommand[] {
  const go = (path: string) => {
    void deps.router.push(path);
  };

  return [
    {
      id: "nav.go-up",
      title: "Go up one folder",
      group: "Navigate",
      keywords: ["parent", "up", "navigator"],
      run: () => deps.ensureNavigatorThenGoUp(),
    },
    {
      id: "nav.open-navigator",
      title: "Open Navigator",
      group: "Navigate",
      keywords: ["browse", "files", "home"],
      run: () => go("/navigator"),
    },
    {
      id: "view.discovery",
      title: "Open Discovery",
      group: "View",
      keywords: ["everything", "search"],
      run: () => go("/discovery"),
    },
    {
      id: "view.providers",
      title: "Open Providers",
      group: "View",
      keywords: ["rclone", "knowledge", "awam"],
      run: () => go("/providers"),
    },
    {
      id: "view.work-groups",
      title: "Open Work Groups",
      group: "View",
      keywords: ["wg", "tabs"],
      run: () => go("/work-groups"),
    },
    {
      id: "view.projects",
      title: "Open Projects",
      group: "View",
      keywords: ["project", "board"],
      run: () => go("/projects"),
    },
    {
      id: "view.extensions",
      title: "Open Extensions",
      group: "View",
      keywords: ["marketplace", "vnext", "plugins"],
      run: () => go("/extensions"),
    },
    {
      id: "view.settings",
      title: "Open Settings",
      group: "View",
      keywords: ["preferences", "core", "options"],
      run: () => go("/settings"),
    },
    {
      id: "view.health",
      title: "Open Health",
      group: "View",
      keywords: ["status", "version", "diagnostics"],
      run: () => go("/health"),
    },
    {
      id: "action.refresh-health",
      title: "Refresh health status",
      group: "Actions",
      keywords: ["health", "reload", "status"],
      run: () => deps.refreshHealth(),
    },
  ];
}
