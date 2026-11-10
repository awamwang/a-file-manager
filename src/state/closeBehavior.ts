// SPDX-License-Identifier: GPL-3.0-or-later
/** Window close → tray / quit / ask (AIA-86). Aligns with system tray + autostart hide. */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "./settingsHydrate";

export type CloseBehavior = "ask" | "tray" | "quit";

export const CLOSE_BEHAVIOR_SETTING_KEY = "core.closeBehavior";

export const closeBehavior = ref<CloseBehavior>("ask");

/** When true, the next CloseRequested must proceed (real quit). */
export const permitWindowClose = ref(false);

export function parseCloseBehavior(value: unknown): CloseBehavior {
  if (value === "tray" || value === "quit" || value === "ask") return value;
  return "ask";
}

/** Sync apply from already-hydrated settings (cold-start critical path). */
export function applyCloseBehaviorFromValues(values: Record<string, unknown>): void {
  closeBehavior.value = parseCloseBehavior(values?.[CLOSE_BEHAVIOR_SETTING_KEY]);
}

export async function loadCloseBehavior(): Promise<CloseBehavior> {
  try {
    const doc = await hydrateSettings();
    applyCloseBehaviorFromValues(doc.values ?? {});
    return closeBehavior.value;
  } catch {
    closeBehavior.value = "ask";
    return "ask";
  }
}

export async function persistCloseBehavior(
  next: CloseBehavior,
): Promise<CloseBehavior> {
  const patch: Record<string, unknown> = {
    [CLOSE_BEHAVIOR_SETTING_KEY]: next,
  };
  const doc = await invoke<SettingsDocument>("settings_set", { patch });
  rememberSettingsDocument(doc);
  const saved = parseCloseBehavior(doc.values?.[CLOSE_BEHAVIOR_SETTING_KEY]);
  closeBehavior.value = saved;
  return saved;
}

/** Hide main window to the existing system tray (same as autostart-hidden). */
export async function hideMainWindowToTray(): Promise<void> {
  await getCurrentWindow().hide();
}

/** Quit the process — same path as tray menu Quit. */
export async function quitApplication(): Promise<void> {
  permitWindowClose.value = true;
  await invoke("app_quit");
}
