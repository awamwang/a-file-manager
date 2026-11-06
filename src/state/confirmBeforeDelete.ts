// SPDX-License-Identifier: GPL-3.0-or-later
/** `power.confirmBeforeDelete` gate + Fluent delete confirm (AIA-185). */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openConfirmDialog } from "./confirmDialog";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "./settingsHydrate";

export const CONFIRM_BEFORE_DELETE_KEY = "power.confirmBeforeDelete";

/** Mirrored setting (default on). */
export const confirmBeforeDelete = ref(true);

export function parseConfirmBeforeDelete(value: unknown): boolean {
  return value !== false;
}

export async function loadConfirmBeforeDelete(): Promise<boolean> {
  try {
    const doc = await hydrateSettings();
    const next = parseConfirmBeforeDelete(doc.values?.[CONFIRM_BEFORE_DELETE_KEY]);
    confirmBeforeDelete.value = next;
    return next;
  } catch {
    confirmBeforeDelete.value = true;
    return true;
  }
}

export async function persistConfirmBeforeDelete(next: boolean): Promise<boolean> {
  const patch: Record<string, unknown> = {
    [CONFIRM_BEFORE_DELETE_KEY]: next,
  };
  const doc = await invoke<SettingsDocument>("settings_set", { patch });
  rememberSettingsDocument(doc);
  const saved = parseConfirmBeforeDelete(doc.values?.[CONFIRM_BEFORE_DELETE_KEY]);
  confirmBeforeDelete.value = saved;
  return saved;
}

export type ConfirmDeleteLabels = {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
};

/**
 * When confirm-before-delete is on, show Fluent modal with 「以后不再提问」.
 * Checking that box and confirming persists `power.confirmBeforeDelete = false`.
 */
export async function confirmDeleteIfNeeded(
  labels: ConfirmDeleteLabels,
): Promise<boolean> {
  const needConfirm = await loadConfirmBeforeDelete();
  if (!needConfirm) return true;

  const result = await openConfirmDialog({
    title: labels.title,
    message: labels.message,
    confirmLabel: labels.confirmLabel,
    cancelLabel: labels.cancelLabel,
    showDontAskAgain: true,
    danger: true,
  });

  if (result.confirmed && result.dontAskAgain) {
    try {
      await persistConfirmBeforeDelete(false);
    } catch {
      // Deletion may still proceed; settings will re-prompt next time.
    }
  }
  return result.confirmed;
}

/** Permanent delete always asks, and does not offer 「以后不再提问」. */
export async function confirmPermanentDelete(
  labels: ConfirmDeleteLabels,
): Promise<boolean> {
  const result = await openConfirmDialog({
    title: labels.title,
    message: labels.message,
    confirmLabel: labels.confirmLabel,
    cancelLabel: labels.cancelLabel,
    showDontAskAgain: false,
    danger: true,
  });
  return result.confirmed;
}
