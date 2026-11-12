// SPDX-License-Identifier: GPL-3.0-or-later
import type { Component } from "vue";

/** Pack 路由 `entry` 到内置懒加载页面的映射。 */
export const builtinEntries: Record<string, () => Promise<Component>> = {
  "builtin:tool-runtime": () => import("../../packs/tool-runtime/ToolRuntimeView.vue"),
};
