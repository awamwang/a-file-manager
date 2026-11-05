// SPDX-License-Identifier: GPL-3.0-or-later
/** Resolve default new file/folder names: settings override → locale fallback. */

import { invoke } from "@tauri-apps/api/core";

type SettingsDocument = {
  values: Record<string, unknown>;
};

export async function resolveNewFolderDefaultName(
  localeFallback: string,
): Promise<string> {
  const fromSettings = await readStringSetting("core.newFolderDefaultName");
  return fromSettings || localeFallback;
}

export async function resolveNewFileDefaultName(
  localeFallback: string,
): Promise<string> {
  const fromSettings = await readStringSetting("core.newFileDefaultName");
  return fromSettings || localeFallback;
}

async function readStringSetting(key: string): Promise<string> {
  try {
    const doc = await invoke<SettingsDocument>("settings_get");
    const raw = doc.values[key];
    return typeof raw === "string" ? raw.trim() : "";
  } catch {
    return "";
  }
}
