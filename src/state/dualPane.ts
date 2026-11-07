// SPDX-License-Identifier: GPL-3.0-or-later
/** Dual-pane Navigator state (AIA-68 + AIA-252 min-width clamp). */

import { computed, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { NavigatorListing } from "./navigatorBrowse";
import { currentPath, navigateTo } from "./navigatorBrowse";
import { clearNavigatorFilter } from "./navigatorFilter";
import { showHiddenItems } from "./showHidden";

export type PaneId = "primary" | "secondary";

export {
  DEFAULT_WINDOW_MIN_HEIGHT_PX,
  DEFAULT_WINDOW_MIN_WIDTH_PX,
  DUAL_PANE_GAP_PX,
  DUAL_PANE_MIN_PANE_PX,
  DUAL_PANE_WINDOW_MIN_WIDTH_PX,
  dualPaneFitsInnerWidth,
  windowMinWidthForDualPane,
} from "./dualPaneLayout";

export const dualPaneEnabled = ref(false);
export const activePane = ref<PaneId>("primary");
export const secondaryPath = ref("");
export const secondaryListing = shallowRef<NavigatorListing | null>(null);
export const secondaryError = ref<string | null>(null);
export const secondaryLoading = ref(false);

export const activePath = computed(() =>
  activePane.value === "secondary" ? secondaryPath.value : currentPath.value,
);

export function toggleDualPane(): void {
  dualPaneEnabled.value = !dualPaneEnabled.value;
  if (dualPaneEnabled.value && !secondaryPath.value && currentPath.value) {
    void navigateSecondary(currentPath.value);
  }
  if (!dualPaneEnabled.value) {
    activePane.value = "primary";
  }
}

export function setActivePane(pane: PaneId): void {
  activePane.value = pane;
}

function browseKey(path: string): string {
  return path.trim().replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
}

export async function navigateSecondary(path: string): Promise<void> {
  const trimmed = path.trim();
  const previous = secondaryPath.value;
  secondaryLoading.value = true;
  secondaryError.value = null;
  try {
    const result = await invoke<NavigatorListing>("navigator_list", {
      path: trimmed,
      showHidden: showHiddenItems.value,
    });
    secondaryListing.value = result;
    secondaryPath.value = result.path;
    if (browseKey(previous) !== browseKey(result.path)) clearNavigatorFilter();
  } catch (err) {
    secondaryListing.value = null;
    secondaryError.value = err instanceof Error ? err.message : String(err);
    secondaryPath.value = trimmed;
  } finally {
    secondaryLoading.value = false;
  }
}

export async function navigateActivePane(path: string): Promise<void> {
  if (activePane.value === "secondary" && dualPaneEnabled.value) {
    await navigateSecondary(path);
  } else {
    await navigateTo(path);
  }
}
