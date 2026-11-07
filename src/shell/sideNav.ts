// SPDX-License-Identifier: GPL-3.0-or-later
/** Primary side-nav model — kept out of AppShell to shrink chrome surface. */

import { AI_ADDON_ENABLED } from "../config/flavor";
import { NAV_ICON_BY_ID, type FluentGlyphId } from "../components/icons/glyphs";

export type SideNavItem = {
  id: string;
  labelKey: string;
  to?: string;
  later?: boolean;
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

export const SIDE_NAV_ITEMS: SideNavItem[] = AI_ADDON_ENABLED
  ? SIDE_NAV_ALL
  : SIDE_NAV_ALL.filter((item) => item.id !== "ai");

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
