// SPDX-License-Identifier: GPL-3.0-or-later

/** Setting ids from Rust `setting_definitions()` — keep in sync with defaults.rs. */
export const SCHEMA_SETTING_KEYS = [
  "core.language",
  "core.theme",
  "core.defaultLocation",
  "core.projectsRoot",
  "core.startupPage",
  "core.showHidden",
  "core.listDensity",
  "core.everythingPreferred",
  "core.extensionsEnabled",
  "core.launchAtStartup",
  "core.launchAtStartupHidden",
  "core.closeBehavior",
  "core.discoveryBlacklist",
  "core.newFolderDefaultName",
  "core.newFileDefaultName",
  "core.fileDoubleClickAction",
  "core.navigatorViewMode",
  "core.navigatorGroupByType",
  "core.newTabBehavior",
  "core.openTool.vscode.enabled",
  "core.openTool.vscode.path",
  "core.openTool.cursor.enabled",
  "core.openTool.cursor.path",
  "core.openTool.trae.enabled",
  "core.openTool.trae.path",
  "core.openTool.qoder.enabled",
  "core.openTool.qoder.path",
  "power.confirmBeforeDelete",
  "power.perPathOverridesEnabled",
  "power.providerPreloadKnowledge",
  "experimental.debugStatusInTitlebar",
  "experimental.previewPane",
  "preview.externalProvider",
] as const;

export type SchemaSettingKey = (typeof SCHEMA_SETTING_KEYS)[number];

/** Enum option values that get localized labels (keyed under settings.fields.<id>.options). */
export const SCHEMA_ENUM_OPTIONS: Record<string, readonly string[]> = {
  "core.language": ["en", "zh-CN", "ja", "de", "fr"],
  "core.theme": ["light", "dark", "system"],
  "core.listDensity": ["standard", "compact"],
  "core.startupPage": ["navigator", "projects", "discovery"],
  "core.closeBehavior": ["ask", "tray", "quit"],
  "core.navigatorViewMode": [
    "iconsLarge",
    "iconsMedium",
    "iconsSmall",
    "list",
    "details",
    "tiles",
    "content",
  ],
  "core.fileDoubleClickAction": [
    "systemDefault",
    "openWithDialog",
    "reveal",
    "vscode",
    "cursor",
    "trae",
    "qoder",
    "none",
  ],
  "core.newTabBehavior": ["guide", "duplicate"],
  "preview.externalProvider": [
    "auto",
    "peek",
    "quicklook",
    "builtin",
    "off",
  ],
};

export function settingFieldMessagePath(
  key: string,
  part: "title" | "description",
): string {
  return `settings.fields.${key}.${part}`;
}

export function settingEnumOptionPath(key: string, option: string): string {
  return `settings.fields.${key}.options.${option}`;
}
