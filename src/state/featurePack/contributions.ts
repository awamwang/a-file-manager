// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Feature Pack 贡献注册表：拉取已启用 Pack 的侧栏/路由/命令，供壳在启动时合并。
 */

import { invoke } from "@tauri-apps/api/core";
import type { RouteRecordRaw, Router } from "vue-router";
import type { AppCommand } from "../../commands/types";
import type { SideNavItem } from "../../shell/sideNav";
import { AI_ADDON_ENABLED } from "../../config/flavor";
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

/** 已通过 addRoute 挂上的 Pack 路由 name，Soft Reload 时卸载不再启用的项。 */
const registeredPackRouteNames = new Set<string>();

function packRouteName(extra: RouteRecordRaw): string | null {
  return typeof extra.name === "string" && extra.name.length > 0
    ? extra.name
    : null;
}

/** 与壳 `router/index.ts` 静态表一致的核心路径（含 AI 门控）。 */
function coreShellRoutePaths(): Set<string> {
  const paths = new Set([
    "/",
    "/health",
    "/navigator",
    "/discovery",
    "/providers",
    "/work-groups",
    "/projects",
    "/extensions",
    "/settings",
    "/settings/health",
    "/settings/shortcuts",
    "/settings/favorites",
    "/settings/open-tools",
  ]);
  if (AI_ADDON_ENABLED) {
    paths.add("/ai");
  }
  return paths;
}

/** 核心路由 + 将于 `builtinEntries` 注册的 Pack 路由路径。 */
function resolvableContributionPaths(): Set<string> {
  const paths = coreShellRoutePaths();
  for (const route of cached.routes) {
    if (!route.path || !builtinEntries[route.entry]) continue;
    paths.add(route.path);
  }
  return paths;
}

function isResolvableNavTo(to: string): boolean {
  return resolvableContributionPaths().has(to);
}

function isResolvableCommandRoute(route: string | null | undefined): boolean {
  if (route == null) return true;
  if (route === "") return false;
  return resolvableContributionPaths().has(route);
}

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

/** 已启用 Pack 的侧栏贡献（含 `order`，由侧栏按序插入）；未知 `to` 不暴露。 */
export function getSideNavExtras(): SideNavItem[] {
  return cached.sideNav
    .filter((item) => item.to && isResolvableNavTo(item.to))
    .map((item) => ({
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
  return cached.commands
    .filter((cmd) => isResolvableCommandRoute(cmd.route))
    .map((cmd) => ({
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

/** 按当前启用贡献同步 Pack 路由：先卸载过期项，再挂上现有项。 */
export function applyFeaturePackRoutes(router: Router): void {
  const extras = getRouteExtras();
  const nextNames = new Set<string>();
  for (const extra of extras) {
    const name = packRouteName(extra);
    if (name) nextNames.add(name);
  }
  for (const name of [...registeredPackRouteNames]) {
    if (!nextNames.has(name)) {
      if (router.hasRoute(name)) {
        router.removeRoute(name);
      }
      registeredPackRouteNames.delete(name);
    }
  }
  const existingPaths = new Set(router.getRoutes().map((route) => route.path));
  for (const extra of extras) {
    const name = packRouteName(extra);
    if (name && router.hasRoute(name)) continue;
    if (typeof extra.path === "string" && existingPaths.has(extra.path)) continue;
    router.addRoute(extra);
    if (typeof extra.path === "string") existingPaths.add(extra.path);
    if (name) registeredPackRouteNames.add(name);
  }
}

/** 从 Host 拉取已启用 Pack 贡献，并合并进侧栏与路由。 */
export async function hydrateFeaturePackContributions(): Promise<void> {
  try {
    cached = normalizeContributions(
      await invoke<AggregatedContributions>("extension_host_list_contributions"),
    );
  } catch {
    // 拉取失败时保留上次贡献，避免 Soft Reload 清空已合并项。
  }
  const { applyFeaturePackSideNav } = await import("../../shell/sideNav");
  applyFeaturePackSideNav();
  const { registerFeaturePackRoutes } = await import("../../router");
  registerFeaturePackRoutes();
}
