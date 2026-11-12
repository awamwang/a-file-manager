// SPDX-License-Identifier: GPL-3.0-or-later
/** `core.showHidden` — Kernel listings omit hidden items unless this is on. */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "../settingsHydrate";
import { readShowHidden } from "./flag";
import { clearListingCache } from "../navigatorListingCache";

export { readShowHidden };

export const showHiddenItems = ref(false);

/** Sync apply from already-hydrated settings (cold-start critical path). */
export function applyShowHiddenFromValues(values: Record<string, unknown>): void {
  const next = readShowHidden(values);
  if (next !== showHiddenItems.value) {
    clearListingCache();
  }
  showHiddenItems.value = next;
}

export async function loadShowHidden(): Promise<boolean> {
  const doc = await hydrateSettings();
  applyShowHiddenFromValues(doc.values ?? {});
  return showHiddenItems.value;
}

export async function setShowHidden(next: boolean): Promise<void> {
  const doc = await invoke<SettingsDocument>("settings_set", {
    patch: { "core.showHidden": next },
  });
  rememberSettingsDocument(doc);
  if (readShowHidden(doc.values) !== showHiddenItems.value) {
    clearListingCache();
  }
  showHiddenItems.value = readShowHidden(doc.values);
}
