// SPDX-License-Identifier: GPL-3.0-or-later
/** Dual-pane 布局开关（AIA-68 + AIA-252）；副窗格 listing 编排在 navigatorBrowse。 */

import { computed, ref } from "vue";
import {
  currentPath,
  navigateSecondary,
  navigateTo,
  secondaryPath,
} from "../navigatorBrowse.ts";

export type PaneId = "primary" | "secondary";

export {
  DEFAULT_WINDOW_MIN_HEIGHT_PX,
  DEFAULT_WINDOW_MIN_WIDTH_PX,
  DUAL_PANE_GAP_PX,
  DUAL_PANE_MIN_PANE_PX,
  DUAL_PANE_WINDOW_MIN_WIDTH_PX,
  dualPaneFitsInnerWidth,
  windowMinWidthForDualPane,
} from "./layout.ts";

export {
  secondaryPath,
  secondaryListing,
  secondaryError,
  secondaryLoading,
  navigateSecondary,
} from "../navigatorBrowse.ts";

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

/**
 * 右键「在第二窗口打开」（FIL-74）：把指定目录送进副栏。
 *
 * - 非双栏状态：先置 `dualPaneEnabled = true` 打开双栏（AppShell 的
 *   `watch(dualPaneEnabled)` 会同步抬高 OS 窗口最小宽度），再导航副栏。
 * - 已是双栏状态：只替换副栏路径，主栏路径 / 标签页 / 导航历史都不动。
 *
 * 与 `toggleDualPane()` 的区别：这里永远只开不关，不会误把已开的双栏关掉。
 */
export async function openInSecondPane(path: string): Promise<void> {
  const target = path.trim();
  if (!target) return;
  dualPaneEnabled.value = true;
  await navigateSecondary(target);
}
