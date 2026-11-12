// SPDX-License-Identifier: GPL-3.0-or-later
/** Floating toast queue (AIA-93) — fixed bottom-right, out of document flow. */

import { ref } from "vue";

export type ToastKind = "info" | "success" | "error";

export type ToastItem = {
  id: number;
  message: string;
  kind: ToastKind;
};

const DEFAULT_MS = 2800;
/** 错误提示默认更久，便于选中复制详细报错。 */
const ERROR_MS = 8000;
const MAX_VISIBLE = 4;

export const toasts = ref<ToastItem[]>([]);

let nextId = 1;

type ToastTimer = {
  remainingMs: number;
  startedAt: number;
  handle: ReturnType<typeof setTimeout> | null;
};

const timers = new Map<number, ToastTimer>();

function clearTimer(id: number): void {
  const entry = timers.get(id);
  if (!entry) return;
  if (entry.handle) clearTimeout(entry.handle);
  timers.delete(id);
}

function armTimer(id: number, remainingMs: number): void {
  clearTimer(id);
  if (remainingMs <= 0) {
    dismissToast(id);
    return;
  }
  const entry: ToastTimer = {
    remainingMs,
    startedAt: Date.now(),
    handle: setTimeout(() => {
      dismissToast(id);
    }, remainingMs),
  };
  timers.set(id, entry);
}

export function dismissToast(id: number): void {
  clearTimer(id);
  toasts.value = toasts.value.filter((item) => item.id !== id);
}

/** 悬停或选中文字时暂停自动消失，方便复制报错内容。 */
export function pauseToast(id: number): void {
  const entry = timers.get(id);
  if (!entry?.handle) return;
  clearTimeout(entry.handle);
  entry.handle = null;
  entry.remainingMs = Math.max(0, entry.remainingMs - (Date.now() - entry.startedAt));
}

export function resumeToast(id: number): void {
  const entry = timers.get(id);
  if (!entry || entry.handle) return;
  armTimer(id, entry.remainingMs);
}

export function showToast(
  message: string,
  kind: ToastKind = "info",
  durationMs?: number,
): number {
  const text = message.trim();
  if (!text) return -1;
  const id = nextId++;
  toasts.value = [...toasts.value, { id, message: text, kind }].slice(-MAX_VISIBLE);
  const ms = durationMs ?? (kind === "error" ? ERROR_MS : DEFAULT_MS);
  if (ms > 0) {
    armTimer(id, ms);
  }
  return id;
}

export function toastSuccess(message: string, durationMs = DEFAULT_MS): number {
  return showToast(message, "success", durationMs);
}

export function toastError(message: string, durationMs = ERROR_MS): number {
  return showToast(message, "error", durationMs);
}

export function toastInfo(message: string, durationMs = DEFAULT_MS): number {
  return showToast(message, "info", durationMs);
}
