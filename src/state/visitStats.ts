// SPDX-License-Identifier: GPL-3.0-or-later
/** Per-path Navigator visit counts + Quick Access ranking (AIA-74). */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type VisitItem = {
  path: string;
  openCount: number;
  lastOpenedAt: number;
  isDir: boolean;
};

export type VisitStatsDocument = {
  visits: VisitItem[];
};

export type VisitRecordResult = {
  document: VisitStatsDocument;
  path: string;
  openCount: number;
};

/** Max rows shown in sidenav Quick Access. */
export const QUICK_ACCESS_CAP = 12;
/** Soft cap mirrored from Rust store prune. */
export const VISIT_STORE_CAP = 200;
/** Recency window (days) for combined score. */
export const RECENCY_WINDOW_DAYS = 30;
/** Recency boost weight vs raw openCount. */
export const RECENCY_WEIGHT = 10;
/** Skip duplicate records for the same path within this window (ms). */
export const VISIT_DEBOUNCE_MS = 400;

export const visitStatsDoc = ref<VisitStatsDocument>({ visits: [] });

let lastRecordKey = "";
let lastRecordAt = 0;

function normalizeKey(path: string): string {
  return path.trim().replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
}

/** Combined score: openCount + decaying recency boost (Files / Explorer Quick Access style). */
export function visitScore(item: VisitItem, nowMs: number): number {
  const ageMs = Math.max(0, nowMs - item.lastOpenedAt);
  const ageDays = ageMs / 86_400_000;
  const recency = Math.max(0, 1 - ageDays / RECENCY_WINDOW_DAYS) * RECENCY_WEIGHT;
  return item.openCount + recency;
}

export function rankQuickAccess(
  visits: VisitItem[],
  nowMs: number,
  cap = QUICK_ACCESS_CAP,
): VisitItem[] {
  return [...visits]
    .sort((a, b) => {
      const d = visitScore(b, nowMs) - visitScore(a, nowMs);
      if (d !== 0) return d;
      if (b.lastOpenedAt !== a.lastOpenedAt) return b.lastOpenedAt - a.lastOpenedAt;
      return a.path.localeCompare(b.path);
    })
    .slice(0, cap);
}

export const quickAccessList = computed(() =>
  rankQuickAccess(visitStatsDoc.value.visits, Date.now(), QUICK_ACCESS_CAP),
);

export async function loadVisitStats(): Promise<VisitStatsDocument> {
  const doc = await invoke<VisitStatsDocument>("visit_stats_get");
  visitStatsDoc.value = doc;
  return doc;
}

/**
 * Record a Navigator directory landing.
 * Count rule: every successful active browse path that is a directory
 * (tab enter/switch/new tab, folder drill-in, address bar, pins) goes through
 * `navigateTo`; that hooks here. Same path within VISIT_DEBOUNCE_MS is skipped.
 */
export async function recordPathVisit(
  path: string,
  isDir = true,
): Promise<VisitRecordResult | null> {
  const trimmed = path.trim();
  if (!trimmed || !isDir) return null;
  const key = normalizeKey(trimmed);
  if (!key) return null;
  const now = Date.now();
  if (key === lastRecordKey && now - lastRecordAt < VISIT_DEBOUNCE_MS) {
    return null;
  }
  lastRecordKey = key;
  lastRecordAt = now;
  try {
    const result = await invoke<VisitRecordResult>("visit_stats_record", {
      path: trimmed,
      isDir,
    });
    visitStatsDoc.value = result.document;
    return result;
  } catch {
    return null;
  }
}

/** Test helper — reset debounce + in-memory doc. */
export function resetVisitStatsForTests(): void {
  visitStatsDoc.value = { visits: [] };
  lastRecordKey = "";
  lastRecordAt = 0;
}
