// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-281: VS Code–like soft "Reload Window".
 * Remounts the Vue shell / KeepAlive views and clears volatile UI caches
 * without wiping persisted data (APPDATA settings, WG/Project hard config,
 * open paths / OpenState).
 */

import { nextTick, ref } from "vue";
import { applyAppLocaleFromValues } from "../i18n";
import { closeCommandPanel } from "./commandPanel";
import { hydrateFeaturePackContributions } from "./featurePack/contributions";
import { refreshActiveListing } from "./navigatorBrowse";
import {
  hydrateSettings,
  invalidateSettingsCache,
} from "./settingsHydrate";
import { clearShellIconQueue } from "./shellIcons";
import { applyUiChromeFromValues } from "./uiChrome";

/** Bump to remount AppShell + KeepAlive views (soft Reload Window). */
export const shellRemountKey = ref(0);

/**
 * Soft Reload Window: clear volatile UI caches, remount shell UI, re-hydrate
 * settings and Feature Pack contributions. Does not restart the process or
 * wipe persisted state.
 */
export async function reloadWindow(): Promise<void> {
  closeCommandPanel();
  clearShellIconQueue();

  invalidateSettingsCache();
  try {
    const doc = await hydrateSettings();
    const values = doc.values ?? {};
    applyUiChromeFromValues(values);
    applyAppLocaleFromValues(values);
  } catch {
    // IPC unavailable (vite-only): still remount UI with prior chrome/locale.
  }

  try {
    await hydrateFeaturePackContributions();
  } catch {
    // IPC unavailable: keep prior side nav / routes / commands.
  }

  shellRemountKey.value += 1;
  await nextTick();
  // Re-bind listing after remount without wiping open paths.
  refreshActiveListing();
}

/** Test helper — remount generation counter. */
export function shellRemountKeyForTests(): number {
  return shellRemountKey.value;
}
