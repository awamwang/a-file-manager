// SPDX-License-Identifier: GPL-3.0-or-later

/** Setting ids from Rust `setting_definitions()` — keep in sync with defaults.rs. */
export const SCHEMA_SETTING_KEYS = [
  "core.language",
  "core.theme",
  "core.defaultLocation",
  "core.showHidden",
  "core.listDensity",
  "core.everythingPreferred",
  "core.extensionsEnabled",
  "power.confirmBeforeDelete",
  "power.perPathOverridesEnabled",
  "experimental.debugStatusInTitlebar",
] as const;

export type SchemaSettingKey = (typeof SCHEMA_SETTING_KEYS)[number];

/** Enum option values that get localized labels (keyed under settings.fields.<id>.options). */
export const SCHEMA_ENUM_OPTIONS: Record<string, readonly string[]> = {
  "core.language": ["en", "zh-CN"],
  "core.theme": ["light", "dark", "system"],
  "core.listDensity": ["standard", "compact"],
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
