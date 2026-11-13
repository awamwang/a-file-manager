// SPDX-License-Identifier: GPL-3.0-or-later
/** Primary side-nav model — kept out of AppShell to shrink chrome surface. */

import { AI_ADDON_ENABLED } from "../config/flavor.ts";
import { NAV_ICON_BY_ID, type FluentGlyphId } from "../components/icons/glyphs.ts";
import { getSideNavExtras } from "../state/featurePack/contributions.ts";

export type SideNavItem = {
  id: string;
  labelKey: string;
  to?: string;
  later?: boolean;
  /** Feature Pack 侧栏插入顺序；内置项按 IA 位置换算。 */
  order?: number;
};

/**
 * Primary pillars first; AI Add-on after Projects (Core Workspace) when flavor includes AI.
 * Discovery/Providers (FS) then Extensions; Health is Settings-only.
 */
const SIDE_NAV_ALL: SideNavItem[] = [
  { id: "navigator", labelKey: "nav.navigator", to: "/navigator" },
  { id: "work-groups", labelKey: "nav.workGroups", to: "/work-groups" },
  { id: "projects", labelKey: "nav.projects", to: "/projects" },
  { id: "ai", labelKey: "nav.ai", to: "/ai" },
  { id: "discovery", labelKey: "nav.discovery", to: "/discovery" },
  { id: "providers", labelKey: "nav.providers", to: "/providers" },
  { id: "extensions", labelKey: "nav.extensions", to: "/extensions" },
  { id: "settings", labelKey: "nav.settings", to: "/settings" },
];

function builtinSideNavItems(): SideNavItem[] {
  return AI_ADDON_ENABLED
    ? SIDE_NAV_ALL
    : SIDE_NAV_ALL.filter((item) => item.id !== "ai");
}

function builtinNavOrder(id: string): number {
  const index = SIDE_NAV_ALL.findIndex((item) => item.id === id);
  return index < 0 ? 0 : index * 10;
}

/** 内置侧栏；hydrate 后含已启用 Feature Pack 贡献。 */
export const SIDE_NAV_ITEMS: SideNavItem[] = [...builtinSideNavItems()];

/** 按 `order` 合并 Pack 侧栏贡献，同 id 保留内置项。 */
export function applyFeaturePackSideNav(): void {
  const builtin = builtinSideNavItems();
  const builtinIds = new Set(builtin.map((item) => item.id));
  const extras = getSideNavExtras().filter((item) => !builtinIds.has(item.id));
  const ranked = [
    ...builtin.map((item) => ({ item, order: builtinNavOrder(item.id) })),
    ...extras.map((item) => ({
      item,
      order: item.order ?? Number.MAX_SAFE_INTEGER,
    })),
  ];
  ranked.sort((a, b) => a.order - b.order || a.item.id.localeCompare(b.item.id));
  SIDE_NAV_ITEMS.splice(0, SIDE_NAV_ITEMS.length, ...ranked.map((entry) => entry.item));
}

export function sideNavIcon(id: string): FluentGlyphId {
  return NAV_ICON_BY_ID[id] ?? "folder";
}

export function isSideNavActive(item: SideNavItem, routePath: string): boolean {
  if (!item.to) return false;
  if (item.to === "/settings") {
    return routePath === "/settings" || routePath.startsWith("/settings/");
  }
  if (item.to === "/navigator") {
    return routePath === "/navigator" || routePath === "/";
  }
  return routePath === item.to || routePath.startsWith(`${item.to}/`);
}
