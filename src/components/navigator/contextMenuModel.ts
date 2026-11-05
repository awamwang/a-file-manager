// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure Navigator context-menu item model (labels supplied by caller / i18n). */

import type { FluentGlyphId } from "../icons/glyphs";
import type { NavigatorEntry } from "../../state/navigatorBrowse";

export type ContextMenuActionId =
  | "open"
  | "openInNewTab"
  | "openWithNative"
  | "openWithIde"
  | "openInTerminal"
  | "toggleFavorite"
  | "systemMenu"
  | "reveal"
  | "copy"
  | "cut"
  | "paste"
  | "rename"
  | "newFolder"
  | "newFile"
  | "copyPath"
  | "properties"
  | "delete";

export type ContextMenuItemDef = {
  id: ContextMenuActionId | string;
  /** i18n key under `contextMenu.*`. */
  labelKey: string;
  enabled: boolean;
  danger?: boolean;
  stub?: boolean;
  separatorBefore?: boolean;
  icon?: FluentGlyphId;
  /** Nested submenu (Sigma-style). */
  children?: ContextMenuItemDef[];
  /** Payload for IDE open. */
  ideId?: string;
};

export type ContextMenuBuildOptions = {
  isFavorite?: boolean;
  canPaste?: boolean;
  /** Empty-area menu (no entry): paste + new folder only. */
  emptyArea?: boolean;
  ideApps?: Array<{ id: string; name: string; available: boolean }>;
};

/** Day-1 visibility rules for a single selected Navigator entry. */
export function buildContextMenuItems(
  entry: NavigatorEntry | null,
  options: ContextMenuBuildOptions = {},
): ContextMenuItemDef[] {
  if (options.emptyArea || !entry) {
    if (!options.emptyArea) return [];
    return [
      {
        id: "paste",
        labelKey: "contextMenu.paste",
        enabled: Boolean(options.canPaste),
        icon: "document",
      },
      {
        id: "newFolder",
        labelKey: "contextMenu.newFolder",
        enabled: true,
        separatorBefore: true,
        icon: "folder",
      },
      {
        id: "newFile",
        labelKey: "contextMenu.newFile",
        enabled: true,
        icon: "document",
      },
    ];
  }
  const isDir = entry.kind === "dir";
  const isFavorite = Boolean(options.isFavorite);
  // AIA-118: IDE/Agent Open with is folder-only. Files keep double-click default + Reveal.
  const ideChildren: ContextMenuItemDef[] = isDir
    ? (options.ideApps ?? [])
        .filter((app) => app.available)
        .map((app) => ({
          id: `openWithIde:${app.id}`,
          labelKey: "contextMenu.openWithIde",
          enabled: true,
          ideId: app.id,
          icon: "puzzle" as FluentGlyphId,
        }))
    : [];

  const items: ContextMenuItemDef[] = [
    { id: "open", labelKey: "contextMenu.open", enabled: true, icon: "folderOpen" },
    {
      id: "openInNewTab",
      labelKey: "contextMenu.openInNewTab",
      enabled: isDir,
      icon: "add",
    },
  ];

  if (isDir) {
    items.push({
      id: "openWith",
      labelKey: "contextMenu.openWith",
      enabled: true,
      icon: "view",
      children: [
        ...ideChildren,
        {
          id: "openWithNative",
          labelKey: "contextMenu.openWithExplorer",
          enabled: true,
          separatorBefore: ideChildren.length > 0,
          icon: "folder",
        },
      ],
    });
  }

  items.push(
    {
      id: "openInTerminal",
      labelKey: "contextMenu.openInTerminal",
      enabled: true,
      icon: "commands",
    },
    {
      id: "toggleFavorite",
      labelKey: isFavorite
        ? "contextMenu.removeFavorite"
        : "contextMenu.addFavorite",
      enabled: true,
      icon: "star",
    },
    {
      id: "systemMenu",
      labelKey: "contextMenu.systemMenu",
      enabled: true,
      icon: "viewAll",
      children: [
        {
          id: "systemMenu",
          labelKey: "contextMenu.systemMenuShow",
          enabled: true,
          icon: "viewAll",
        },
      ],
    },
    { id: "reveal", labelKey: "contextMenu.reveal", enabled: true, icon: "eye" },
  );

  return [
    ...items,
    {
      id: "copy",
      labelKey: "contextMenu.copy",
      enabled: true,
      separatorBefore: true,
      icon: "document",
    },
    { id: "cut", labelKey: "contextMenu.cut", enabled: true, icon: "document" },
    {
      id: "paste",
      labelKey: "contextMenu.paste",
      enabled: Boolean(options.canPaste),
      icon: "document",
    },
    { id: "rename", labelKey: "contextMenu.rename", enabled: true, icon: "color" },
    { id: "newFolder", labelKey: "contextMenu.newFolder", enabled: true, icon: "folder" },
    { id: "newFile", labelKey: "contextMenu.newFile", enabled: true, icon: "document" },
    { id: "copyPath", labelKey: "contextMenu.copyPath", enabled: true, icon: "globe" },
    {
      id: "properties",
      labelKey: "contextMenu.properties",
      enabled: true,
      icon: "settings",
    },
    {
      id: "delete",
      labelKey: "contextMenu.delete",
      enabled: true,
      danger: true,
      separatorBefore: true,
      icon: "close",
    },
  ];
}

/** True when Open in new tab should run (directory selection only). */
export function canOpenInNewTab(entry: NavigatorEntry | null): boolean {
  return entry?.kind === "dir";
}
