// SPDX-License-Identifier: GPL-3.0-or-later
/** Ask before replacing an existing name. Apply-to-all is per-paste-batch only.
 *
 * CF-003 / TR-014 (`docs/file-ops-scenario-baseline.md` §已拍板 2026-11-22):
 * a checked "Apply to all" box remembers the user's choice **inside the
 * current paste operation only**. The next paste re-prompts. We deliberately
 * do NOT write to localStorage or persist across sessions — Explorer's "Do this
 * for all" is per-dialog, not per-app, and silently inheriting a stale default
 * is the kind of surprise that wipes data on a multi-conflict paste weeks
 * later. */

import { ref } from "vue";

export type NameConflictChoice = "replace" | "skip" | "keep";
export type NameConflictResult = {
  choice: NameConflictChoice | "cancel";
};

export const nameConflictOpen = ref(false);
export const nameConflictName = ref("");
export const nameConflictApplyToAll = ref(false);
// TR-013 / CF-006: the destination exists but is a different type (file↔folder).
// When true the dialog renders a stronger "replaces the folder/file" warning.
export const nameConflictTypeMismatch = ref(false);

// Per-batch state. Cleared by `resetNameConflictBatch()` at the start of each
// `transferEntries` call; never persisted anywhere.
let rememberedChoice: NameConflictChoice | null = null;

let pendingResolve: ((result: NameConflictResult) => void) | null = null;

function settle(result: NameConflictResult): void {
  const resolve = pendingResolve;
  pendingResolve = null;
  nameConflictOpen.value = false;
  nameConflictName.value = "";
  nameConflictApplyToAll.value = false;
  nameConflictTypeMismatch.value = false;
  resolve?.(result);
}

/** Call at the top of every paste batch. The remembered "Apply to all" choice
 *  does not survive past this reset. */
export function resetNameConflictBatch(): void {
  rememberedChoice = null;
}

/** Exposed for tests; production code does not need it. */
export function currentRememberedChoice(): NameConflictChoice | null {
  return rememberedChoice;
}

export function openNameConflictDialog(name: string, typeMismatch = false): Promise<NameConflictResult> {
  if (pendingResolve) settle({ choice: "cancel" });
  nameConflictName.value = name;
  nameConflictTypeMismatch.value = typeMismatch;
  nameConflictApplyToAll.value = false;
  nameConflictOpen.value = true;
  return new Promise((resolve) => {
    pendingResolve = resolve;
  });
}

export function setNameConflictApplyToAll(on: boolean): void {
  nameConflictApplyToAll.value = on;
}

export function resolveNameConflict(choice: NameConflictChoice | "cancel"): void {
  if (!nameConflictOpen.value) return;
  if (choice !== "cancel" && nameConflictApplyToAll.value) {
    rememberedChoice = choice;
  }
  settle({ choice });
}

/** Within the same batch, honor the remembered choice; otherwise ask. */
export async function resolveNameConflictChoice(
  name: string,
  typeMismatch = false,
): Promise<NameConflictChoice | "cancel"> {
  if (rememberedChoice) return rememberedChoice;
  const result = await openNameConflictDialog(name, typeMismatch);
  return result.choice;
}
