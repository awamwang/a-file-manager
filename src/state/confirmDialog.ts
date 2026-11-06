// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Promise-based Fluent confirm dialog (AIA-185).
 * Host: `ConfirmDialogHost.vue` — reusable for delete and other confirmations.
 */

import { ref } from "vue";

export type ConfirmDialogRequest = {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Show 「以后不再提问」. Default false; delete flow opts in. */
  showDontAskAgain?: boolean;
  /** Accent confirm button styled as destructive. */
  danger?: boolean;
};

export type ConfirmDialogResult = {
  confirmed: boolean;
  dontAskAgain: boolean;
};

export const confirmDialogOpen = ref(false);
export const confirmDialogRequest = ref<ConfirmDialogRequest | null>(null);
export const confirmDialogDontAskAgain = ref(false);
export const confirmDialogBusy = ref(false);

let pendingResolve: ((result: ConfirmDialogResult) => void) | null = null;

function settle(result: ConfirmDialogResult): void {
  const resolve = pendingResolve;
  pendingResolve = null;
  confirmDialogOpen.value = false;
  confirmDialogBusy.value = false;
  confirmDialogRequest.value = null;
  confirmDialogDontAskAgain.value = false;
  resolve?.(result);
}

/** Open centered Fluent confirm; resolves when the user confirms or cancels. */
export function openConfirmDialog(
  request: ConfirmDialogRequest,
): Promise<ConfirmDialogResult> {
  if (pendingResolve) {
    // Replace in-flight dialog as cancelled so callers never hang.
    settle({ confirmed: false, dontAskAgain: false });
  }
  confirmDialogRequest.value = request;
  confirmDialogDontAskAgain.value = false;
  confirmDialogBusy.value = false;
  confirmDialogOpen.value = true;
  return new Promise<ConfirmDialogResult>((resolve) => {
    pendingResolve = resolve;
  });
}

export function resolveConfirmDialog(confirmed: boolean): void {
  if (!confirmDialogOpen.value) return;
  settle({
    confirmed,
    dontAskAgain: confirmDialogDontAskAgain.value,
  });
}

export function cancelConfirmDialog(): void {
  if (confirmDialogBusy.value) return;
  resolveConfirmDialog(false);
}
