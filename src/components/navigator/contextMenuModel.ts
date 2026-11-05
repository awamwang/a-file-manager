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
  ideApps?: Array<{ id: string; name: string; available: boolean }>;
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
  const ideChildren: ContextMenuItemDef[] = (options.ideApps ?? [])
    .filter((app) => app.available)
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
    { id: "open", labelKey: "contextMenu.open", enabled: true, icon: "folderOpen" },
    {
      id: "openInNewTab",
      labelKey: "contextMenu.openInNewTab",
      enabled: isDir,
      icon: "add",
    },
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
          id: "systemMenu",
          labelKey: "contextMenu.systemMenuShow",
          enabled: true,
          icon: "viewAll",
        },
      ],
    },
    { id: "reveal", labelKey: "contextMenu.reveal", enabled: true, icon: "eye" },
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

function isZipEntry(entry: NavigatorEntry): boolean {
  if (entry.kind !== "file") return false;
  const name = entry.name.toLowerCase();
  return name.endsWith(".zip");
}

/** True when Open in new tab should run (directory selection only). */
export function canOpenInNewTab(entry: NavigatorEntry | null): boolean {
  return entry?.kind === "dir";
}
