// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 设置页展示归类（FIL-59）。
 * Schema 键仍全部保留；此处只控制侧栏分区内的分组、排序与默认隐藏。
 */

export type SettingsUiGroupId =
  | "appearance"
  | "startup"
  | "files"
  | "features"
  | "safety"
  | "advanced"
  | "preview"
  | "other";

/** 仍写入 schema/document，但设置页默认不展示（能力不丢）。 */
export const SETTINGS_UI_HIDDEN_KEYS = [
  // Discovery 页才是黑名单权威编辑入口。
  "core.discoveryBlacklist",
  // Providers 页已有网关地址编辑，避免双入口。
  "power.awamGatewayUrl",
  // 标题栏调试相位对日常使用噪音大；键仍保留供诊断。
  "experimental.debugStatusInTitlebar",
] as const;

const GROUP_ORDER: SettingsUiGroupId[] = [
  "appearance",
  "startup",
  "files",
  "features",
  "safety",
  "advanced",
  "preview",
  "other",
];

const KEY_GROUP: Record<string, SettingsUiGroupId> = {
  "core.language": "appearance",
  "core.theme": "appearance",
  "core.listDensity": "appearance",
  "core.navigatorViewMode": "appearance",
  "core.navigatorGroupByType": "appearance",

  "core.startupPage": "startup",
  "core.defaultLocation": "startup",
  "core.launchAtStartup": "startup",
  "core.launchAtStartupHidden": "startup",
  "core.closeBehavior": "startup",

  "core.showHidden": "files",
  "core.newTabBehavior": "files",
  "core.fileDoubleClickAction": "files",
  "core.newFolderDefaultName": "files",
  "core.newFileDefaultName": "files",
  "core.projectsRoot": "files",

  "core.everythingPreferred": "features",
  "core.extensionsEnabled": "features",

  "power.confirmBeforeDelete": "safety",
  "power.perPathOverridesEnabled": "advanced",
  "power.providerPreloadKnowledge": "advanced",

  "experimental.previewPane": "preview",
  "preview.externalProvider": "preview",
};

export function settingsUiGroupForKey(key: string): SettingsUiGroupId {
  return KEY_GROUP[key] ?? "other";
}

export function settingsUiGroupOrder(): readonly SettingsUiGroupId[] {
  return GROUP_ORDER;
}

export function isSettingsUiHiddenKey(key: string): boolean {
  return (SETTINGS_UI_HIDDEN_KEYS as readonly string[]).includes(key);
}

export function settingsUiGroupMessagePath(group: SettingsUiGroupId): string {
  return `settings.groups.${group}`;
}

export function compareSettingsUiKeys(a: string, b: string): number {
  const ga = GROUP_ORDER.indexOf(settingsUiGroupForKey(a));
  const gb = GROUP_ORDER.indexOf(settingsUiGroupForKey(b));
  if (ga !== gb) return ga - gb;
  return a.localeCompare(b);
}
