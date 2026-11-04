// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure Navigator context-menu item model (labels supplied by caller / i18n). */

import type { NavigatorEntry } from "../../state/navigatorBrowse";

export type ContextMenuActionId =
  | "open"
  | "openInNewTab"
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

/** Day-1 visibility rules for a single selected Navigator entry. */
export function buildContextMenuItems(
  entry: NavigatorEntry | null,
): ContextMenuItemDef[] {
  if (!entry) return [];
  const isDir = entry.kind === "dir";
  return [
    { id: "open", labelKey: "contextMenu.open", enabled: true },
    {
      id: "openInNewTab",
      labelKey: "contextMenu.openInNewTab",
      // Real (AIA-54): directories open a new Navigator tab at that path via newNavigatorTab.
      enabled: isDir,
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
