// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure Navigator context-menu item model (labels supplied by caller / i18n). */

import type { NavigatorEntry } from "../../state/navigatorBrowse";

export type ContextMenuActionId =
  | "open"
  | "openInNewTab"
  | "openWith"
  | "openInTerminal"
  | "toggleFavorite"
  | "systemMenu"
  | "reveal"
  | "copyPath"
  | "delete";

export type ContextMenuItemDef = {
  id: ContextMenuActionId;
  /** i18n key under `contextMenu.*`. */
  labelKey: string;
  enabled: boolean;
  danger?: boolean;
  stub?: boolean;
};

export type ContextMenuBuildOptions = {
  isFavorite?: boolean;
};

/** Day-1 visibility rules for a single selected Navigator entry. */
export function buildContextMenuItems(
  entry: NavigatorEntry | null,
  options: ContextMenuBuildOptions = {},
): ContextMenuItemDef[] {
  if (!entry) return [];
  const isDir = entry.kind === "dir";
  const isFavorite = Boolean(options.isFavorite);
  return [
    { id: "open", labelKey: "contextMenu.open", enabled: true },
    {
      id: "openInNewTab",
      labelKey: "contextMenu.openInNewTab",
      enabled: isDir,
    },
    {
      id: "openWith",
      labelKey: "contextMenu.openWith",
      // Native Open With dialog — files only (Sigma-aligned).
      enabled: !isDir,
    },
    {
      id: "openInTerminal",
      labelKey: "contextMenu.openInTerminal",
      // Files open their parent directory (Sigma-aligned).
      enabled: true,
    },
    {
      id: "toggleFavorite",
      labelKey: isFavorite
        ? "contextMenu.removeFavorite"
        : "contextMenu.addFavorite",
      enabled: true,
    },
    {
      id: "systemMenu",
      labelKey: "contextMenu.systemMenu",
      enabled: true,
    },
    { id: "reveal", labelKey: "contextMenu.reveal", enabled: true },
    { id: "copyPath", labelKey: "contextMenu.copyPath", enabled: true },
    {
      id: "delete",
      labelKey: "contextMenu.delete",
      enabled: true,
      danger: true,
      stub: true,
    },
  ];
}

/** True when Open in new tab should run (directory selection only). */
export function canOpenInNewTab(entry: NavigatorEntry | null): boolean {
  return entry?.kind === "dir";
}
