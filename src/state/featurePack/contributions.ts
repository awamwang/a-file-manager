// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Feature Pack 贡献注册表：拉取已启用 Pack 的侧栏/路由/命令，供壳在启动时合并。
 */

import { invoke } from "@tauri-apps/api/core";
import type { RouteRecordRaw, Router } from "vue-router";
import type { AppCommand } from "../../commands/types";
import type { SideNavItem } from "../../shell/sideNav";
import { builtinEntries } from "./builtinEntries";

export type FeaturePackSideNavContribution = {
  id: string;
  labelKey: string;
  to: string;
  order: number;
};

export type FeaturePackRouteContribution = {
  path: string;
  name: string;
  entry: string;
};

export type FeaturePackCommandContribution = {
  id: string;
  labelKey: string;
  route?: string | null;
};

export type FeaturePackSidecarContribution = {
  id: string;
  command: string;
  args: string[];
  workingDirRel?: string | null;
};

export type AggregatedContributions = {
  sideNav: FeaturePackSideNavContribution[];
  routes: FeaturePackRouteContribution[];
  commands: FeaturePackCommandContribution[];
  sidecars: FeaturePackSidecarContribution[];
};

const EMPTY_CONTRIBUTIONS: AggregatedContributions = {
  sideNav: [],
  routes: [],
  commands: [],
  sidecars: [],
};

let cached: AggregatedContributions = EMPTY_CONTRIBUTIONS;

function normalizeContributions(
  raw: Partial<AggregatedContributions> | null | undefined,
): AggregatedContributions {
  return {
    sideNav: Array.isArray(raw?.sideNav) ? raw.sideNav : [],
    routes: Array.isArray(raw?.routes) ? raw.routes : [],
    commands: Array.isArray(raw?.commands) ? raw.commands : [],
    sidecars: Array.isArray(raw?.sidecars) ? raw.sidecars : [],
  };
}

/** 已启用 Pack 的侧栏贡献（含 `order`，由侧栏按序插入）。 */
export function getSideNavExtras(): SideNavItem[] {
  return cached.sideNav.map((item) => ({
    id: item.id,
    labelKey: item.labelKey,
    to: item.to,
    order: item.order,
  }));
}

/** 仅当 `entry` 在 builtinEntries 有映射时生成可注册路由。 */
export function getRouteExtras(): RouteRecordRaw[] {
  const extras: RouteRecordRaw[] = [];
  for (const item of cached.routes) {
    const loader = builtinEntries[item.entry];
    if (!loader || !item.path) continue;
    extras.push({
      path: item.path,
      name: item.name,
      component: loader,
    });
  }
  return extras;
}

/** 对齐命令面板条目：打开贡献中的 `route`。 */
export function getCommandExtras(deps: {
  router: Pick<Router, "push">;
  t: (key: string) => string;
}): AppCommand[] {
  return cached.commands.map((cmd) => ({
    id: cmd.id,
    title: deps.t(cmd.labelKey),
    group: "View",
    keywords: [cmd.id, cmd.route ?? ""].filter((token) => token.length > 0),
    run: () => {
      if (cmd.route) {
        void deps.router.push(cmd.route);
      }
    },
  }));
}

/** 从 Host 拉取已启用 Pack 贡献，并合并进侧栏与路由。 */
export async function hydrateFeaturePackContributions(): Promise<void> {
  try {
    cached = normalizeContributions(
      await invoke<AggregatedContributions>("extension_host_list_contributions"),
    );
  } catch {
    cached = normalizeContributions(undefined);
  }
  const { applyFeaturePackSideNav } = await import("../../shell/sideNav");
  applyFeaturePackSideNav();
  const { registerFeaturePackRoutes } = await import("../../router");
  registerFeaturePackRoutes();
}
