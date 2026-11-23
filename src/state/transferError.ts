// SPDX-License-Identifier: GPL-3.0-or-later
/** Per-item transfer error dialog (LC-006 / LC-007 / LC-008).
 *
 * 已拍板 2026-11-22 (LC-007): when one item of a multi-file copy/move fails for a
 * retryable reason — file in use, access denied, destination disk full — Windows
 * Explorer shows a **Retry / Skip / Abort** dialog for that item instead of
 * silently killing the whole batch. The backend classifies the IO failure into an
 * error class (`file_ops/mod.rs::classify_transfer_error`); this module owns the
 * per-item dialog lifecycle.
 *
 * The dialog state is per-invocation and in-memory only: there is NO "apply to
 * all" remembered choice and NOTHING is persisted across operations or sessions —
 * every failing item asks again, matching Explorer.
 *
 * The class string names MUST stay in sync with the backend constants
 * `ERROR_CLASS_IN_USE` / `ERROR_CLASS_ACCESS_DENIED` / `ERROR_CLASS_DISK_FULL`.
 */

import { ref } from "vue";

export type TransferErrorClass = "in-use" | "access-denied" | "disk-full";
export type TransferErrorChoice = "retry" | "skip" | "abort";

/** Classes that get the Retry / Skip / Abort dialog. Anything else is a hard
 *  refusal (folder-into-itself, volume root, validation) where retry/skip is
 *  meaningless and the whole batch must abort. */
const RETRYABLE_CLASSES: ReadonlySet<string> = new Set<TransferErrorClass>([
  "in-use",
  "access-denied",
  "disk-full",
]);

export function isRetryableTransferErrorClass(
  errorClass: string | null | undefined,
): errorClass is TransferErrorClass {
  return typeof errorClass === "string" && RETRYABLE_CLASSES.has(errorClass);
}

export const transferErrorOpen = ref(false);
export const transferErrorClass = ref<TransferErrorClass | "">("");
export const transferErrorName = ref("");

let pendingResolve: ((choice: TransferErrorChoice) => void) | null = null;

function settle(choice: TransferErrorChoice): void {
  const resolve = pendingResolve;
  pendingResolve = null;
  transferErrorOpen.value = false;
  transferErrorClass.value = "";
  transferErrorName.value = "";
  resolve?.(choice);
}

export function openTransferErrorDialog(
  errorClass: TransferErrorClass,
  name: string,
): Promise<TransferErrorChoice> {
  if (pendingResolve) settle("abort");
  transferErrorClass.value = errorClass;
  transferErrorName.value = name;
  transferErrorOpen.value = true;
  return new Promise((resolve) => {
    pendingResolve = resolve;
  });
}

export function resolveTransferError(choice: TransferErrorChoice): void {
  if (!transferErrorOpen.value) return;
  settle(choice);
}

/** Opens the dialog and waits for the user's Retry / Skip / Abort choice. */
export async function resolveTransferErrorChoice(
  errorClass: TransferErrorClass,
  name: string,
): Promise<TransferErrorChoice> {
  return openTransferErrorDialog(errorClass, name);
}

/** Exposed for tests; production code does not need it. */
export function transferErrorPending(): boolean {
  return pendingResolve !== null;
}
