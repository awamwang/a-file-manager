// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 设置空字符串含义「使用生效默认」的键：UI 展示真实生效值，文案用（默认：x）。
 */

import { invoke } from "@tauri-apps/api/core";
import { appConfigDir, join } from "@tauri-apps/api/path";

/** 存空时回退到运行时/语言默认的字符串设置键。 */
export const SETTINGS_EMPTY_MEANS_DEFAULT_KEYS = [
  "core.defaultLocation",
  "core.projectsRoot",
  "core.newFolderDefaultName",
  "core.newFileDefaultName",
] as const;

export type SettingsEmptyMeansDefaultKey =
  (typeof SETTINGS_EMPTY_MEANS_DEFAULT_KEYS)[number];

export function isSettingsEmptyMeansDefaultKey(
  key: string,
): key is SettingsEmptyMeansDefaultKey {
  return (SETTINGS_EMPTY_MEANS_DEFAULT_KEYS as readonly string[]).includes(key);
}

export type SettingsEffectiveDefaults = Partial<
  Record<SettingsEmptyMeansDefaultKey, string>
>;

function toWindowsPath(path: string): string {
  return path.replace(/\//g, "\\");
}

/**
 * 解析空值时的真实生效默认（USERPROFILE、项目根、当前语言新建名等）。
 * 与当前已保存的覆盖值无关，始终返回「空=默认」时的生效值。
 */
export async function loadSettingsEffectiveDefaults(opts: {
  newFolderLocaleDefault: string;
  newFileLocaleDefault: string;
}): Promise<SettingsEffectiveDefaults> {
  const out: SettingsEffectiveDefaults = {
    "core.newFolderDefaultName": opts.newFolderLocaleDefault,
    "core.newFileDefaultName": opts.newFileLocaleDefault,
  };

  try {
    const home = await invoke<string>("navigator_default_path");
    if (home.trim()) out["core.defaultLocation"] = home;
  } catch {
    // 保留未解析；UI 仍可展示已配置值。
  }

  try {
    const configDir = await appConfigDir();
    const projectsRoot = toWindowsPath(await join(configDir, "projects"));
    if (projectsRoot.trim()) out["core.projectsRoot"] = projectsRoot;
  } catch {
    // ignore
  }

  return out;
}

/** 输入框展示值：已配置非空用配置，否则用生效默认。 */
export function displaySettingsStringValue(
  stored: unknown,
  key: string,
  effective: SettingsEffectiveDefaults,
): string {
  const raw =
    typeof stored === "string" ? stored : stored == null ? "" : String(stored);
  if (raw.trim()) return raw;
  if (isSettingsEmptyMeansDefaultKey(key)) {
    return effective[key] ?? "";
  }
  return raw;
}

/** 保存时若与生效默认相同则写回空串，保持「跟随默认」语义。 */
export function normalizeSettingsStringForSave(
  key: string,
  value: string,
  effective: SettingsEffectiveDefaults,
): string {
  if (!isSettingsEmptyMeansDefaultKey(key)) return value;
  const eff = effective[key];
  if (eff != null && value.trim() === eff.trim()) return "";
  return value;
}
