// SPDX-License-Identifier: GPL-3.0-or-later
/** Windows-style Navigator view modes (Explorer-aligned). */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { hydrateSettings, rememberSettingsDocument } from "./settingsHydrate";

export type NavigatorViewMode =
  | "iconsLarge"
  | "iconsMedium"
  | "iconsSmall"
  | "list"
  | "details";

export const NAVIGATOR_VIEW_MODES: readonly NavigatorViewMode[] = [
  "iconsLarge",
  "iconsMedium",
  "iconsSmall",
  "list",
  "details",
] as const;

/** Shell icon / thumbnail edge length for each view. */
export const VIEW_ICON_SIZE: Record<NavigatorViewMode, number> = {
  iconsLarge: 96,
  iconsMedium: 64,
  iconsSmall: 48,
  list: 16,
  details: 16,
};

export const navigatorViewMode = ref<NavigatorViewMode>("details");

export const isIconView = computed(() =>
  navigatorViewMode.value.startsWith("icons"),
);

export const isDetailsView = computed(() => navigatorViewMode.value === "details");

export const isListView = computed(() => navigatorViewMode.value === "list");

let loadPromise: Promise<void> | null = null;

function coerceMode(value: unknown): NavigatorViewMode {
  if (typeof value === "string" && (NAVIGATOR_VIEW_MODES as readonly string[]).includes(value)) {
    return value as NavigatorViewMode;
  }
  return "details";
}

export async function loadNavigatorViewMode(): Promise<NavigatorViewMode> {
  if (!loadPromise) {
    loadPromise = (async () => {
      try {
        const doc = await hydrateSettings();
        navigatorViewMode.value = coerceMode(doc.values["core.navigatorViewMode"]);
      } catch {
        navigatorViewMode.value = "details";
      }
    })().finally(() => {
      loadPromise = null;
    });
  }
  await loadPromise;
  return navigatorViewMode.value;
}

export async function setNavigatorViewMode(mode: NavigatorViewMode): Promise<void> {
  navigatorViewMode.value = mode;
  try {
    const doc = await invoke<{ version: number; values: Record<string, unknown> }>(
      "settings_set",
      {
        patch: { "core.navigatorViewMode": mode },
      },
    );
    rememberSettingsDocument(doc);
  } catch {
    // Keep local mode even if persist fails; next load may revert.
  }
}

export function viewModeLabelKey(mode: NavigatorViewMode): string {
  switch (mode) {
    case "iconsLarge":
      return "chrome.viewIconsLarge";
    case "iconsMedium":
      return "chrome.viewIconsMedium";
    case "iconsSmall":
      return "chrome.viewIconsSmall";
    case "list":
      return "chrome.viewList";
    case "details":
      return "chrome.viewDetails";
  }
}
