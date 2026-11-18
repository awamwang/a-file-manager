// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 密钥短显通道（ADR 0016）：明文只存在模块级非响应式 Map 中，
 * 响应式状态只暴露「是否短显中」的开关——明文不进 Vue 响应式状态、不进任何 store。
 * 5 秒自动回掩码；面板渲染时经 revealedValueOf() 直读非响应文本。
 */

import { ref } from "vue";

/** 短显时长（毫秒）：到期自动回掩码。 */
export const SECRET_REVEAL_TTL_MS = 5000;

/** 响应式：处于短显中的条目集合（不含明文，只有 entry key）。 */
const revealedEntries = ref<ReadonlySet<string>>(new Set());
/** 非响应式：entry → 明文（仅短显期间持有）。 */
const revealedValues = new Map<string, string>();
/** 每条目一个回收定时器。 */
const revealTimers = new Map<string, ReturnType<typeof setTimeout>>();

export function isSecretRevealed(entry: string): boolean {
  return revealedEntries.value.has(entry);
}

/** 渲染期直读明文（仅在 isSecretRevealed 为真时调用）。 */
export function revealedValueOf(entry: string): string {
  return revealedValues.get(entry) ?? "";
}

/** 开启/重置一条短显：明文写入非响应式 Map，5 秒后自动清除。 */
export function revealSecretTransient(
  entry: string,
  value: string,
  ttlMs: number = SECRET_REVEAL_TTL_MS,
): void {
  if (!entry) return;
  revealedValues.set(entry, value);
  const next = new Set(revealedEntries.value);
  next.add(entry);
  revealedEntries.value = next;
  const prevTimer = revealTimers.get(entry);
  if (prevTimer !== undefined) clearTimeout(prevTimer);
  revealTimers.set(
    entry,
    setTimeout(() => {
      revealedValues.delete(entry);
      const cleared = new Set(revealedEntries.value);
      cleared.delete(entry);
      revealedEntries.value = cleared;
      revealTimers.delete(entry);
    }, ttlMs),
  );
}

/** 提前回掩码（面板关闭等场景）。 */
export function maskSecretReveal(entry: string): void {
  const timer = revealTimers.get(entry);
  if (timer !== undefined) clearTimeout(timer);
  if (!revealedEntries.value.has(entry)) return;
  revealedValues.delete(entry);
  const cleared = new Set(revealedEntries.value);
  cleared.delete(entry);
  revealedEntries.value = cleared;
}
