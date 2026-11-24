// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator selection set with multi-select helpers (AIA-64). */

import { computed, ref } from "vue";
import type { NavigatorEntry } from "./navigatorBrowse.ts";
import { normalizePathKey } from "./pathKey.ts";

export const selectedPaths = ref<string[]>([]);
/** Anchor path for Shift+click ranges. */
export const selectionAnchor = ref<string | null>(null);

export const primarySelectedPath = computed(() => selectedPaths.value[0] ?? null);

export function selectOnly(path: string): void {
  selectedPaths.value = [path];
  selectionAnchor.value = path;
}

export function clearSelection(): void {
  selectedPaths.value = [];
  selectionAnchor.value = null;
}

export function isSelected(path: string): boolean {
  return selectedPaths.value.includes(path);
}

export function toggleInSelection(path: string): void {
  if (selectedPaths.value.includes(path)) {
    selectedPaths.value = selectedPaths.value.filter((p) => p !== path);
  } else {
    selectedPaths.value = [...selectedPaths.value, path];
  }
  selectionAnchor.value = path;
}

export function selectRange(
  orderedPaths: string[],
  fromPath: string | null,
  toPath: string,
): void {
  const toIndex = orderedPaths.indexOf(toPath);
  if (toIndex < 0) {
    selectOnly(toPath);
    return;
  }
  const fromIndex = fromPath ? orderedPaths.indexOf(fromPath) : toIndex;
  if (fromIndex < 0) {
    selectOnly(toPath);
    return;
  }
  const start = Math.min(fromIndex, toIndex);
  const end = Math.max(fromIndex, toIndex);
  selectedPaths.value = orderedPaths.slice(start, end + 1);
  // Keep the range origin unless none was set yet.
  if (!selectionAnchor.value || !orderedPaths.includes(selectionAnchor.value)) {
    selectionAnchor.value = fromPath && orderedPaths.includes(fromPath) ? fromPath : toPath;
  }
}

export function setSelection(paths: string[], anchor?: string | null): void {
  selectedPaths.value = [...paths];
  selectionAnchor.value = anchor ?? paths[0] ?? null;
}

/** SEL-010 (已拍板 2026-11-22): Esc 三层优先级 — rename inline 中 → 取消
 * rename（由重命名输入框自身的 keydown 拦截，优先级最高）；有选中 → 清空
 * 选中；两者皆无 → 不消费，交给全局处理器。 */
export function escSemantics(
  renameActive: boolean,
  selectionCount: number,
): "cancel-rename" | "clear-selection" | "pass-through" {
  if (renameActive) return "cancel-rename";
  if (selectionCount > 0) return "clear-selection";
  return "pass-through";
}

export function selectedEntries(
  entries: NavigatorEntry[] | null | undefined,
): NavigatorEntry[] {
  if (!entries?.length || !selectedPaths.value.length) return [];
  const set = new Set(selectedPaths.value);
  return entries.filter((e) => set.has(e.path));
}

/**
 * 删除后保持选中（Explorer 习惯）：
 * 在可见顺序中定位被删条目的最后一个位置，选中其后的第一个剩余条目；
 * 已删到列表末尾则回退到剩余最后一项；目录清空或被删路径不在当前
 * 可见列表（如跨目录操作）时返回 null，由调用方自行清除选中。
 * 必须在移除 listing 条目之前用删除前的顺序调用。
 */
export function nextSelectionAfterRemoval(
  orderedPaths: string[],
  removedPaths: string[],
): string | null {
  if (!orderedPaths.length || !removedPaths.length) return null;
  const removed = new Set(
    removedPaths
      .map((path) => normalizePathKey(path))
      .filter((key) => key.length > 0),
  );
  if (!removed.size) return null;
  let lastIndex = -1;
  for (let i = 0; i < orderedPaths.length; i++) {
    if (removed.has(normalizePathKey(orderedPaths[i] ?? ""))) lastIndex = i;
  }
  if (lastIndex < 0) return null;
  for (let i = lastIndex + 1; i < orderedPaths.length; i++) {
    if (!removed.has(normalizePathKey(orderedPaths[i] ?? ""))) return orderedPaths[i];
  }
  for (let i = lastIndex - 1; i >= 0; i--) {
    if (!removed.has(normalizePathKey(orderedPaths[i] ?? ""))) return orderedPaths[i];
  }
  return null;
}
