// SPDX-License-Identifier: GPL-3.0-or-later
/** Primary side-nav model — kept out of AppShell to shrink chrome surface. */

import { NAV_ICON_BY_ID, type FluentGlyphId } from "../components/icons/glyphs";

export type SideNavItem = {
  id: string;
  labelKey: string;
  to?: string;
  later?: boolean;
};

/**
 * Primary pillars first; AI Add-on after Projects (Core Workspace).
 * Discovery/Providers (FS) then Extensions; Health is Settings-only.
 */
export const SIDE_NAV_ITEMS: SideNavItem[] = [
  { id: "navigator", labelKey: "nav.navigator", to: "/navigator" },
  { id: "work-groups", labelKey: "nav.workGroups", to: "/work-groups" },
  { id: "projects", labelKey: "nav.projects", to: "/projects" },
  { id: "ai", labelKey: "nav.ai", to: "/ai" },
  { id: "discovery", labelKey: "nav.discovery", to: "/discovery" },
  { id: "providers", labelKey: "nav.providers", to: "/providers" },
  { id: "extensions", labelKey: "nav.extensions", to: "/extensions" },
  { id: "settings", labelKey: "nav.settings", to: "/settings" },
];

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
