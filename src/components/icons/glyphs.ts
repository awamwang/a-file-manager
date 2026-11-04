// SPDX-License-Identifier: GPL-3.0-or-later
/** Curated Fluent metaphor catalog: Segoe codepoint + SVG fallback paths (16×16). */

export type FluentGlyphId =
  | "back"
  | "forward"
  | "up"
  | "view"
  | "commands"
  | "minimize"
  | "maximize"
  | "restore"
  | "close"
  | "folder"
  | "folderOpen"
  | "search"
  | "cloud"
  | "viewAll"
  | "board"
  | "puzzle"
  | "settings"
  | "heart"
  | "document"
  | "globe"
  | "color"
  | "density"
  | "eye"
  | "shield"
  | "flash"
  | "bug"
  | "home"
  | "add"
  | "check";

export type FluentGlyph = {
  /** Segoe Fluent Icons / MDL2 private-use codepoint */
  codepoint: string;
  /** SVG path `d` attributes for 16×16 viewBox */
  paths: string[];
};

export const FLUENT_GLYPHS: Record<FluentGlyphId, FluentGlyph> = {
  back: {
    codepoint: "\uE72B",
    paths: ["M10.5 3.2 4.7 8l5.8 4.8V10h4V6h-4V3.2z"],
  },
  forward: {
    codepoint: "\uE72A",
    paths: ["M5.5 3.2 11.3 8 5.5 12.8V10h-4V6h4V3.2z"],
  },
  up: {
    codepoint: "\uE74A",
    paths: ["M8 3.5 3.2 8.3h3.3v4.2h2.8V8.3h3.3L8 3.5z"],
  },
  view: {
    codepoint: "\uE890",
    paths: [
      "M2 4.5h5v3H2v-3zm7 0h5v3H9v-3zM2 9h5v3H2V9zm7 0h5v3H9V9z",
    ],
  },
  commands: {
    codepoint: "\uE756",
    paths: [
      "M3 3.5h10v9H3v-9zm1.2 1.5v1.2h2.2L4.2 9.2l.9.9 2.8-3.7H9.5V5H4.2z",
    ],
  },
  minimize: {
    codepoint: "\uE921",
    paths: ["M3.5 7.25h9v1.5h-9v-1.5z"],
  },
  maximize: {
    codepoint: "\uE922",
    paths: ["M3.5 3.5h9v9h-9v-9zm1.25 1.25v6.5h6.5v-6.5h-6.5z"],
  },
  restore: {
    codepoint: "\uE923",
    paths: [
      "M5.5 3.5h7v7h-1.25V4.75H5.5V3.5zM3.5 5.5h7v7h-7v-7zm1.25 1.25v4.5h4.5v-4.5h-4.5z",
    ],
  },
  close: {
    codepoint: "\uE8BB",
    paths: [
      "M4.1 3.9 3.4 4.6 7.05 8.25 3.4 11.9l.7.7 3.65-3.65 3.65 3.65.7-.7-3.65-3.65L12.1 4.6l-.7-.7-3.65 3.65L4.1 3.9z",
    ],
  },
  folder: {
    codepoint: "\uE8B7",
    paths: [
      "M2.5 4.5h4l1.2 1.2H13.5v7.3H2.5V4.5zm1.2 2.4v5h8.6v-5H3.7z",
    ],
  },
  folderOpen: {
    codepoint: "\uE838",
    paths: [
      "M2 4.2h4.2l1 1.1H11v1.2H4.4L2.7 12.3h10.6l1.2-5.2H5.2L2 4.2z",
    ],
  },
  search: {
    codepoint: "\uE721",
    paths: [
      "M7 2.8a4.2 4.2 0 1 0 2.5 7.5l2.6 2.6.9-.9-2.6-2.6A4.2 4.2 0 0 0 7 2.8zm0 1.3a2.9 2.9 0 1 1 0 5.8 2.9 2.9 0 0 1 0-5.8z",
    ],
  },
  cloud: {
    codepoint: "\uE753",
    paths: [
      "M5.2 6.2a2.8 2.8 0 0 1 5.4-.8A2.6 2.6 0 0 1 12.5 10H4.2a2.3 2.3 0 0 1 1-3.8z",
    ],
  },
  viewAll: {
    codepoint: "\uE8A9",
    paths: [
      "M2.5 3.5h4v4h-4v-4zm7 0h4v4h-4v-4zm-7 7h4v4h-4v-4zm7 0h4v4h-4v-4z",
    ],
  },
  board: {
    codepoint: "\uE8FD",
    paths: [
      "M2.5 3h11v10h-11V3zm1.3 1.3v7.4h3.2V4.3H3.8zm4.5 0v4.5h4.2V4.3H8.3zm0 5.8v1.6h4.2v-1.6H8.3z",
    ],
  },
  puzzle: {
    codepoint: "\uEA86",
    paths: [
      "M6.2 2.8v1.6H4.5a1.5 1.5 0 0 0 0 3h1.7v1.2H4.5a1.5 1.5 0 0 0 0 3h1.7v1.6h1.5V11.6h1.2a1.5 1.5 0 0 0 0-3H7.7V7.4h1.2a1.5 1.5 0 0 0 0-3H7.7V2.8H6.2z",
    ],
  },
  settings: {
    codepoint: "\uE713",
    paths: [
      "M8 5.2a2.8 2.8 0 1 0 0 5.6 2.8 2.8 0 0 0 0-5.6zm0 1.3a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3zM7.2 2.4h1.6l.3 1.5.9.4 1.3-.8 1.1 1.1-.8 1.3.4.9 1.5.3v1.6l-1.5.3-.4.9.8 1.3-1.1 1.1-1.3-.8-.9.4-.3 1.5H7.2l-.3-1.5-.9-.4-1.3.8-1.1-1.1.8-1.3-.4-.9L2.5 8.8V7.2l1.5-.3.4-.9-.8-1.3 1.1-1.1 1.3.8.9-.4.3-1.5z",
    ],
  },
  heart: {
    codepoint: "\uEB52",
    paths: [
      "M8 13.2 3.4 8.4A3.2 3.2 0 0 1 8 3.9a3.2 3.2 0 0 1 4.6 4.5L8 13.2z",
    ],
  },
  document: {
    codepoint: "\uE8A5",
    paths: [
      "M4 2.5h5.2L12 5.3V13.5H4V2.5zm1.2 1.2v8.6h5.6V6.2H8.2V3.7H5.2z",
    ],
  },
  globe: {
    codepoint: "\uE774",
    paths: [
      "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zm0 1.2c.6 0 1.5 1.7 1.7 4.3H6.3C6.5 5.4 7.4 3.7 8 3.7zm-2.9.9c-.5.7-.9 1.9-1 3.4h2C6.2 5.8 6.6 4.7 7.1 4.6zm5.8 0c.5.1.9 1.2 1.1 2.9h-2c-.1-1.5-.5-2.7-1-3.4.3.1.6.3.9.5zM4.1 9.2c.1 1.5.5 2.7 1 3.4-.3-.1-.6-.3-.9-.5-.5-.7-.9-1.9-1.1-3.4h1zm2.2 0h3.4c-.2 2.6-1.1 4.3-1.7 4.3s-1.5-1.7-1.7-4.3zm4.5 0h2c-.2 1.5-.6 2.7-1.1 3.4-.3.2-.6.4-.9.5.5-.7.9-1.9 1-3.9z",
    ],
  },
  color: {
    codepoint: "\uE790",
    paths: [
      "M8 2.5a5.5 5.5 0 0 0 0 11c1.2 0 1.8-.7 1.8-1.6 0-.4-.2-.8-.5-1.1-.3-.3-.5-.7-.5-1.1 0-.9.7-1.6 1.6-1.6H11a3.5 3.5 0 0 0 0-7H8zm-2.2 2a1.1 1.1 0 1 1 0 2.2 1.1 1.1 0 0 1 0-2.2zm2.4 0a1.1 1.1 0 1 1 0 2.2 1.1 1.1 0 0 1 0-2.2zm2.4 0a1.1 1.1 0 1 1 0 2.2 1.1 1.1 0 0 1 0-2.2zM5.2 8a1.1 1.1 0 1 1 0 2.2 1.1 1.1 0 0 1 0-2.2z",
    ],
  },
  density: {
    codepoint: "\uE8FD",
    paths: [
      "M2.5 3.5h11v1.4H2.5V3.5zm0 3.8h11v1.4H2.5V7.3zm0 3.8h11v1.4H2.5v-1.4z",
    ],
  },
  eye: {
    codepoint: "\uE7B3",
    paths: [
      "M8 3.5C4.2 3.5 1.5 8 1.5 8s2.7 4.5 6.5 4.5S14.5 8 14.5 8 11.8 3.5 8 3.5zM8 5.2A2.8 2.8 0 1 1 8 10.8 2.8 2.8 0 0 1 8 5.2zm0 1.3a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z",
    ],
  },
  shield: {
    codepoint: "\uEA18",
    paths: [
      "M8 2.4 3.2 4.2v3.6c0 3.2 2.1 5.4 4.8 6.3 2.7-.9 4.8-3.1 4.8-6.3V4.2L8 2.4zm0 1.4 3.5 1.3v2.7c0 2.2-1.3 3.8-3.5 4.6-2.2-.8-3.5-2.4-3.5-4.6V5.1L8 3.8z",
    ],
  },
  flash: {
    codepoint: "\uE945",
    paths: ["M9.2 2.5 4.5 9.2h3.1L6.8 13.5 12.2 6.5H9.1L9.2 2.5z"],
  },
  bug: {
    codepoint: "\uEBE8",
    paths: [
      "M7.2 2.8h1.6v1.2h.8a2.2 2.2 0 0 1 2.2 2.2v.6h1.4v1.3h-1.4v1.2h1.4v1.3h-1.4v.6A2.2 2.2 0 0 1 9.6 14H6.4a2.2 2.2 0 0 1-2.2-2.2v-.6H2.8V10h1.4V8.8H2.8V7.5h1.4v-.6A2.2 2.2 0 0 1 6.4 4h.8V2.8zm.2 2.5h1.2A.9.9 0 0 1 9.5 6.2v5.6a.9.9 0 0 1-.9.9H6.6a.9.9 0 0 1-.9-.9V6.2a.9.9 0 0 1 .9-.9h.9z",
    ],
  },
  home: {
    codepoint: "\uE80F",
    paths: ["M8 2.8 2.5 7.5V13h4.2V9.5h2.6V13h4.2V7.5L8 2.8z"],
  },
  add: {
    codepoint: "\uE710",
    paths: ["M7.25 3.5h1.5v3.75H12.5v1.5H8.75V12.5h-1.5V8.75H3.5v-1.5h3.75V3.5z"],
  },
  check: {
    codepoint: "\uE73E",
    paths: ["M6.6 11.4 3.2 8l1-1 2.4 2.4 5.2-5.2 1 1-6.2 6.2z"],
  },
};

export const NAV_ICON_BY_ID: Record<string, FluentGlyphId> = {
  navigator: "folderOpen",
  discovery: "search",
  providers: "cloud",
  "work-groups": "viewAll",
  projects: "board",
  extensions: "puzzle",
  settings: "settings",
  health: "heart",
};

export const SETTINGS_ICON_BY_KEY: Record<string, FluentGlyphId> = {
  "core.language": "globe",
  "core.theme": "color",
  "core.defaultLocation": "home",
  "core.showHidden": "eye",
  "core.listDensity": "density",
  "core.everythingPreferred": "search",
  "core.extensionsEnabled": "puzzle",
  "core.launchAtStartup": "flash",
  "core.launchAtStartupHidden": "eye",
  "core.discoveryBlacklist": "search",
  "power.confirmBeforeDelete": "shield",
  "power.perPathOverridesEnabled": "flash",
  "experimental.debugStatusInTitlebar": "bug",
};

export function settingsIconForKey(key: string): FluentGlyphId {
  return SETTINGS_ICON_BY_KEY[key] ?? "settings";
}

export const COMMAND_GROUP_ICON: Record<string, FluentGlyphId> = {
  Navigate: "folderOpen",
  View: "view",
  Actions: "flash",
};

export function commandGroupIcon(group: string): FluentGlyphId {
  return COMMAND_GROUP_ICON[group] ?? "commands";
}

export function entryKindIcon(isDir: boolean): FluentGlyphId {
  return isDir ? "folder" : "document";
}
