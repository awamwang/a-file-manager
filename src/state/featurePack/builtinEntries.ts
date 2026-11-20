// SPDX-License-Identifier: GPL-3.0-or-later
import type { Component } from "vue";

/** Pack 路由/分区 `entry` 到内置懒加载组件的映射。 */
export const builtinEntries: Record<string, () => Promise<Component>> = {
  "builtin:tool-runtime": () => import("../../packs/tool-runtime/ToolRuntimeView.vue"),
};

/** 按 entry 获取组件加载器；ADR-0023 扩展分区也走此表。 */
export function resolveBuiltinEntry(entry: string): (() => Promise<Component>) | undefined {
  return builtinEntries[entry];
}
