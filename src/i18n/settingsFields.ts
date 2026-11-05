// SPDX-License-Identifier: GPL-3.0-or-later

/** Setting ids from Rust `setting_definitions()` ? keep in sync with defaults.rs. */
export const SCHEMA_SETTING_KEYS = [
  "core.language",
  "core.theme",
  "core.defaultLocation",
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
  "power.confirmBeforeDelete",
  "power.perPathOverridesEnabled",
  "experimental.debugStatusInTitlebar",
  "experimental.previewPane",
] as const;

export type SchemaSettingKey = (typeof SCHEMA_SETTING_KEYS)[number];

/** Enum option values that get localized labels (keyed under settings.fields.<id>.options). */
export const SCHEMA_ENUM_OPTIONS: Record<string, readonly string[]> = {
  "core.language": ["en", "zh-CN", "ja", "de", "fr"],
  "core.theme": ["light", "dark", "system"],
  "core.listDensity": ["standard", "compact"],
  "core.startupPage": ["navigator", "projects", "discovery"],
  "core.closeBehavior": ["ask", "tray", "quit"],
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
