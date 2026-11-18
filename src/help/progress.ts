// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Local onboarding progress (ADR 0021, decision R2-2).
 *
 * Only one localStorage key is used. Replaying a tour never writes progress;
 * resetting progress removes exactly that one key and nothing else.
 */

const STORAGE_KEY = "afm.help.progress.v1";

export type HelpProgress = {
  seen: string[];
  done: string[];
};

const EMPTY_PROGRESS: HelpProgress = { seen: [], done: [] };

let cached: HelpProgress | null = null;
let storageUnavailable = false;

function clone(progress: HelpProgress): HelpProgress {
  return { seen: [...progress.seen], done: [...progress.done] };
}

/** Read the progress document; corrupt or unavailable storage means "empty". */
function read(): HelpProgress {
  if (cached) return cached;
  if (storageUnavailable) return clone(EMPTY_PROGRESS);
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      cached = clone(EMPTY_PROGRESS);
      return cached;
    }
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      cached = clone(EMPTY_PROGRESS);
      return cached;
    }
    const record = parsed as Record<string, unknown>;
    cached = {
      seen: Array.isArray(record["seen"])
        ? record["seen"].filter((item): item is string => typeof item === "string")
        : [],
      done: Array.isArray(record["done"])
        ? record["done"].filter((item): item is string => typeof item === "string")
        : [],
    };
    return cached;
  } catch {
    storageUnavailable = true;
    return clone(EMPTY_PROGRESS);
  }
}

/** Persist the whole document; one key, no other settings touched. */
function write(progress: HelpProgress): void {
  cached = clone(progress);
  if (storageUnavailable) return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(cached));
  } catch {
    storageUnavailable = true;
  }
}

/** Mark a hint/tour as shown once. */
export function markSeen(id: string): void {
  if (!id) return;
  const current = read();
  if (current.seen.includes(id)) return;
  write({ ...current, seen: [...current.seen, id] });
}

/** Mark a tour as completed (skip counts as completion, so it will not auto-replay). */
export function markDone(id: string): void {
  if (!id) return;
  const current = read();
  if (current.done.includes(id)) return;
  write({ ...current, done: [...current.done, id] });
}

/** Whether a hint/tour was already shown. */
export function isSeen(id: string): boolean {
  return read().seen.includes(id);
}

/** Whether a tour was completed or dismissed. */
export function isDone(id: string): boolean {
  return read().done.includes(id);
}

/** True until any guidance progress has been recorded. */
export function hasAnyProgress(): boolean {
  const current = read();
  return current.seen.length > 0 || current.done.length > 0;
}

/**
 * Clear onboarding progress only. Deliberately never calls clear() and never
 * touches any other localStorage key, setting, file or user data.
 */
export function resetAllProgress(): void {
  cached = clone(EMPTY_PROGRESS);
  if (storageUnavailable) return;
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    storageUnavailable = true;
  }
}
