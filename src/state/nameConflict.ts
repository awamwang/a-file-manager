// SPDX-License-Identifier: GPL-3.0-or-later
/** Ask before replacing an existing name. Remember replace/skip for later pastes. */

import { ref } from "vue";

export type NameConflictChoice = "replace" | "skip" | "keep";
export type NameConflictDefault = "ask" | "replace" | "skip";

export type NameConflictResult = {
  choice: NameConflictChoice | "cancel";
};

const STORAGE_KEY = "afm.nameConflictDefault";

export const nameConflictOpen = ref(false);
export const nameConflictName = ref("");
export const nameConflictRememberReplace = ref(false);
export const nameConflictRememberSkip = ref(false);

let pendingResolve: ((result: NameConflictResult) => void) | null = null;

export function readNameConflictDefault(): NameConflictDefault {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "replace" || raw === "skip") return raw;
  } catch {
    /* private mode */
  }
  return "ask";
}

export function writeNameConflictDefault(value: NameConflictDefault): void {
  try {
    if (value === "ask") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, value);
  } catch {
    /* ignore quota / private mode */
  }
}

function settle(result: NameConflictResult): void {
  const resolve = pendingResolve;
  pendingResolve = null;
  nameConflictOpen.value = false;
  nameConflictName.value = "";
  nameConflictRememberReplace.value = false;
  nameConflictRememberSkip.value = false;
  resolve?.(result);
}

export function openNameConflictDialog(name: string): Promise<NameConflictResult> {
  if (pendingResolve) settle({ choice: "cancel" });
  nameConflictName.value = name;
  nameConflictRememberReplace.value = false;
  nameConflictRememberSkip.value = false;
  nameConflictOpen.value = true;
  return new Promise((resolve) => {
    pendingResolve = resolve;
  });
}

export function setNameConflictRemember(which: "replace" | "skip", on: boolean): void {
  if (which === "replace") {
    nameConflictRememberReplace.value = on;
    if (on) nameConflictRememberSkip.value = false;
  } else {
    nameConflictRememberSkip.value = on;
    if (on) nameConflictRememberReplace.value = false;
  }
}

export function resolveNameConflict(choice: NameConflictChoice | "cancel"): void {
  if (!nameConflictOpen.value) return;
  if (choice !== "cancel") {
    if (nameConflictRememberReplace.value) writeNameConflictDefault("replace");
    else if (nameConflictRememberSkip.value) writeNameConflictDefault("skip");
  }
  settle({ choice });
}

/** Use the remembered default, otherwise ask. Cancel aborts the whole transfer. */
export async function resolveNameConflictChoice(
  name: string,
): Promise<NameConflictChoice | "cancel"> {
  const remembered = readNameConflictDefault();
  if (remembered === "replace" || remembered === "skip") return remembered;
  const result = await openNameConflictDialog(name);
  return result.choice;
}
