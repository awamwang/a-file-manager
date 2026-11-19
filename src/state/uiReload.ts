// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-281: VS Code–like soft "Reload Window".
 * Remounts the Vue shell / KeepAlive views and clears volatile UI caches
 * without wiping persisted data (APPDATA settings, WG/Project hard config,
 * open paths / OpenState).
 */

import { nextTick, ref } from "vue";
import { applyAppLocaleFromValues } from "../i18n/index.ts";
import { closeCommandPanel } from "./commandPanel.ts";
import { hydrateFeaturePackContributions } from "./featurePack/contributions.ts";
import { refreshActiveListing } from "./navigatorBrowse.ts";
import {
  hydrateSettings,
  invalidateSettingsCache,
} from "./settingsHydrate.ts";
import { clearShellIconQueue } from "./shellIcons.ts";
import { applyUiChromeFromValues } from "./uiChrome.ts";

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

  // Pack 禁用/卸载后当前路径可能已从路由表移除，回退到扩展页避免空白。
  try {
    const { default: router } = await import("../router/index.ts");
    const path = router.currentRoute.value.path;
    if (router.resolve(path).matched.length === 0) {
      await router.replace("/extensions");
    }
  } catch {
    // 路由不可用时仍继续 remount。
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

/**
 * 仅重取 Feature Pack 贡献并做路由回退，**不** remount 整个 shell。
 * 用于安装/启用/禁用/卸载后同步侧栏/路由/命令——贡献已是响应式，无需整窗重载，避免闪烁抖动。
 */
export async function reloadContributions(): Promise<void> {
  try {
    await hydrateFeaturePackContributions();
  } catch {
    // IPC 不可用时保留上次贡献。
  }
  // 当前路径可能因禁用/卸载其所属 Pack 而失效 → 回退扩展页，避免空白（不 remount）。
  try {
    const { default: router } = await import("../router/index.ts");
    const path = router.currentRoute.value.path;
    if (router.resolve(path).matched.length === 0) {
      await router.replace("/extensions");
    }
  } catch {
    // 路由不可用时忽略。
  }
}
