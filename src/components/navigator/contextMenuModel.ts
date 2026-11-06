// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure Navigator context-menu item model (labels supplied by caller / i18n). */

import type { FluentGlyphId } from "../icons/glyphs";
import type { NavigatorEntry } from "../../state/navigatorBrowse";

export type ContextMenuActionId =
  | "open"
  | "openInNewTab"
  | "openWithNative"
  | "openWithSystemDefault"
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
  | "compressZip"
  | "extractZip"
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
  /** Payload for terminal open. */
  terminalId?: string;
  elevated?: boolean;
};

export type ContextMenuBuildOptions = {
  isFavorite?: boolean;
  canPaste?: boolean;
  /** Empty-area menu (no entry): paste + new folder only. */
  emptyArea?: boolean;
  /** Recycle Bin listing: restore instead of normal file verbs. */
  recycleMode?: boolean;
  ideApps?: Array<{ id: string; name: string; available: boolean; enabled?: boolean }>;
  terminals?: Array<{
    id: string;
    name: string;
    available: boolean;
    supportsElevation?: boolean;
  }>;
};

/** Day-1 visibility rules for a single selected Navigator entry. */
export function buildContextMenuItems(
  entry: NavigatorEntry | null,
  options: ContextMenuBuildOptions = {},
): ContextMenuItemDef[] {
  if (options.recycleMode) {
    if (!entry) return [];
    return [
      {
        id: "restore",
        labelKey: "recycleBin.restore",
        enabled: true,
        icon: "folderOpen",
      },
      {
        id: "properties",
        labelKey: "contextMenu.properties",
        enabled: true,
        icon: "view",
      },
    ];
  }
  if (options.emptyArea || !entry) {
    if (!options.emptyArea) return [];
    // Paste lives in the top toolbar; empty-area list is New only.
    return [
      {
        id: "newFolder",
        labelKey: "contextMenu.newFolder",
        enabled: true,
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
  const ideChildren: ContextMenuItemDef[] = (options.ideApps ?? [])
    .filter((app) => app.available && app.enabled !== false)
    .map((app) => ({
      id: `openWithIde:${app.id}`,
      labelKey: "contextMenu.openWithIde",
      enabled: true,
      ideId: app.id,
      icon: "puzzle" as FluentGlyphId,
    }));

  const terminalChildren: ContextMenuItemDef[] = [];
  for (const term of (options.terminals ?? []).filter((t) => t.available)) {
    terminalChildren.push({
      id: `terminal:${term.id}`,
      labelKey: "contextMenu.openInTerminalApp",
      enabled: true,
      terminalId: term.id,
      elevated: false,
      icon: "commands",
    });
    if (term.supportsElevation !== false) {
      terminalChildren.push({
        id: `terminal:${term.id}:admin`,
        labelKey: "contextMenu.openInTerminalAdmin",
        enabled: true,
        terminalId: term.id,
        elevated: true,
        icon: "shield",
      });
    }
  }
  if (terminalChildren.length === 0) {
    terminalChildren.push({
      id: "openInTerminal",
      labelKey: "contextMenu.openInTerminalDefault",
      enabled: true,
      icon: "commands",
    });
  }

  const openWithChildren: ContextMenuItemDef[] = [];
  // Folders: new-tab lives under Open with (no top-level "Open current tab").
  if (isDir) {
    openWithChildren.push({
      id: "openInNewTab",
      labelKey: "contextMenu.openInNewTab",
      enabled: true,
      icon: "add",
    });
  }
  if (!isDir) {
    openWithChildren.push({
      id: "openWithSystemDefault",
      labelKey: "contextMenu.openWithSystemDefault",
      enabled: true,
      icon: "folderOpen",
    });
    openWithChildren.push({
      id: "openWithNative",
      labelKey: "contextMenu.openWithChoose",
      enabled: true,
      icon: "view",
    });
  }
  if (ideChildren.length) {
    openWithChildren.push(
      ...ideChildren.map((child, index) => ({
        ...child,
        separatorBefore: index === 0 && openWithChildren.length > 0,
      })),
    );
  }
  if (isDir) {
    openWithChildren.push({
      id: "openWithNative",
      labelKey: "contextMenu.openWithExplorer",
      enabled: true,
      separatorBefore: openWithChildren.length > 0,
      icon: "folder",
    });
  }

  return [
    {
      id: "openWith",
      labelKey: "contextMenu.openWith",
      enabled: openWithChildren.length > 0,
      icon: "view",
      children: openWithChildren,
    },
    {
      id: "openInTerminal",
      labelKey: "contextMenu.openInTerminal",
      enabled: true,
      icon: "commands",
      children: terminalChildren,
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
          id: "systemMenuShow",
          labelKey: "contextMenu.systemMenuShow",
          enabled: true,
          icon: "viewAll",
        },
      ],
    },
    // Clipboard / rename / copy-path live in the Windows-style top toolbar.
    { id: "newFolder", labelKey: "contextMenu.newFolder", enabled: true, separatorBefore: true, icon: "folder" },
    { id: "newFile", labelKey: "contextMenu.newFile", enabled: true, icon: "document" },
    {
      id: "compressZip",
      labelKey: "contextMenu.compressZip",
      enabled: true,
      separatorBefore: true,
      icon: "document",
    },
    {
      id: "extractZip",
      labelKey: "contextMenu.extractZip",
      enabled: isZipEntry(entry),
      icon: "folderOpen",
    },
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

/** Win11-style icon strip: Cut / Copy / Paste / Rename / Copy path. */
export function buildContextToolbarItems(
  options: ContextMenuBuildOptions = {},
): ContextMenuItemDef[] {
  const entryMode = !options.emptyArea;
  return [
    {
      id: "cut",
      labelKey: "contextMenu.cut",
      enabled: entryMode,
      icon: "cut",
    },
    {
      id: "copy",
      labelKey: "contextMenu.copy",
      enabled: entryMode,
      icon: "copy",
    },
    {
      id: "paste",
      labelKey: "contextMenu.paste",
      enabled: Boolean(options.canPaste),
      icon: "paste",
    },
    {
      id: "rename",
      labelKey: "contextMenu.rename",
      enabled: entryMode,
      icon: "rename",
    },
    {
      id: "copyPath",
      labelKey: "contextMenu.copyPath",
      enabled: entryMode,
      icon: "link",
    },
  ];
}

function isZipEntry(entry: NavigatorEntry): boolean {
  if (entry.kind !== "file") return false;
  const name = entry.name.toLowerCase();
  return name.endsWith(".zip");
}

/** True when Open in new tab should run (directory selection only). */
export function canOpenInNewTab(entry: NavigatorEntry | null): boolean {
  return entry?.kind === "dir";
}
