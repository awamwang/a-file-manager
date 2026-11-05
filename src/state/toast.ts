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
const MAX_VISIBLE = 4;

export const toasts = ref<ToastItem[]>([]);

let nextId = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export function dismissToast(id: number): void {
  const timer = timers.get(id);
  if (timer) {
    clearTimeout(timer);
    timers.delete(id);
  }
  toasts.value = toasts.value.filter((item) => item.id !== id);
}

export function showToast(
  message: string,
  kind: ToastKind = "info",
  durationMs = DEFAULT_MS,
): number {
  const text = message.trim();
  if (!text) return -1;
  const id = nextId++;
  toasts.value = [...toasts.value, { id, message: text, kind }].slice(-MAX_VISIBLE);
  if (durationMs > 0) {
    timers.set(
      id,
      setTimeout(() => {
        dismissToast(id);
      }, durationMs),
    );
  }
  return id;
}

export function toastSuccess(message: string, durationMs = DEFAULT_MS): number {
  return showToast(message, "success", durationMs);
}

export function toastError(message: string, durationMs = DEFAULT_MS): number {
  return showToast(message, "error", durationMs);
}

export function toastInfo(message: string, durationMs = DEFAULT_MS): number {
  return showToast(message, "info", durationMs);
}
