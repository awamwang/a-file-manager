// SPDX-License-Identifier: GPL-3.0-or-later
/** Dual-pane 布局开关（AIA-68 + AIA-252）；副窗格 listing 编排在 navigatorBrowse。 */

import { computed, ref } from "vue";
import {
  currentPath,
  navigateSecondary,
  navigateTo,
  secondaryPath,
} from "../navigatorBrowse";

export type PaneId = "primary" | "secondary";

export {
  DEFAULT_WINDOW_MIN_HEIGHT_PX,
  DEFAULT_WINDOW_MIN_WIDTH_PX,
  DUAL_PANE_GAP_PX,
  DUAL_PANE_MIN_PANE_PX,
  DUAL_PANE_WINDOW_MIN_WIDTH_PX,
  dualPaneFitsInnerWidth,
  windowMinWidthForDualPane,
} from "./layout";

export {
  secondaryPath,
  secondaryListing,
  secondaryError,
  secondaryLoading,
  navigateSecondary,
} from "../navigatorBrowse";

export const dualPaneEnabled = ref(false);
export const activePane = ref<PaneId>("primary");

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

export async function navigateActivePane(path: string): Promise<void> {
  if (activePane.value === "secondary" && dualPaneEnabled.value) {
    await navigateSecondary(path);
  } else {
    await navigateTo(path);
  }
}
