// SPDX-License-Identifier: GPL-3.0-or-later
/** `core.showHidden` — Kernel listings omit hidden items unless this is on. */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "./settingsHydrate";
import { readShowHidden } from "./showHiddenFlag";

export { readShowHidden };

export const showHiddenItems = ref(false);

export async function loadShowHidden(): Promise<boolean> {
  const doc = await hydrateSettings();
  showHiddenItems.value = readShowHidden(doc.values);
  return showHiddenItems.value;
}

export async function setShowHidden(next: boolean): Promise<void> {
  const doc = await invoke<SettingsDocument>("settings_set", {
    patch: { "core.showHidden": next },
  });
  rememberSettingsDocument(doc);
  showHiddenItems.value = readShowHidden(doc.values);
}
