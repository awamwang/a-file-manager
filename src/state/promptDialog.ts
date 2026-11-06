// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Promise-based Fluent prompt dialog (AIA-186).
 * Host: `PromptDialogHost.vue` — replaces window.prompt for name entry.
 */

import { ref } from "vue";

export type PromptDialogRequest = {
  title: string;
  /** Optional helper / description under the title. */
  message?: string;
  /** Accessible label for the text field. */
  label?: string;
  defaultValue?: string;
  placeholder?: string;
  confirmLabel?: string;
  cancelLabel?: string;
};

export type PromptDialogResult = {
  confirmed: boolean;
  value: string;
};

export const promptDialogOpen = ref(false);
export const promptDialogRequest = ref<PromptDialogRequest | null>(null);
export const promptDialogValue = ref("");
export const promptDialogBusy = ref(false);

let pendingResolve: ((result: PromptDialogResult) => void) | null = null;

function settle(result: PromptDialogResult): void {
  const resolve = pendingResolve;
  pendingResolve = null;
  promptDialogOpen.value = false;
  promptDialogBusy.value = false;
  promptDialogRequest.value = null;
  promptDialogValue.value = "";
  resolve?.(result);
}

/** Open centered Fluent prompt; resolves when the user confirms or cancels. */
export function openPromptDialog(
  request: PromptDialogRequest,
): Promise<PromptDialogResult> {
  if (pendingResolve) {
    // Replace in-flight dialog as cancelled so callers never hang.
    settle({ confirmed: false, value: "" });
  }
  promptDialogRequest.value = request;
  promptDialogValue.value = request.defaultValue ?? "";
  promptDialogBusy.value = false;
  promptDialogOpen.value = true;
  return new Promise<PromptDialogResult>((resolve) => {
    pendingResolve = resolve;
  });
}

export function resolvePromptDialog(confirmed: boolean): void {
  if (!promptDialogOpen.value) return;
  settle({
    confirmed,
    value: promptDialogValue.value,
  });
}

export function cancelPromptDialog(): void {
  if (promptDialogBusy.value) return;
  resolvePromptDialog(false);
}
