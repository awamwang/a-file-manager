// SPDX-License-Identifier: GPL-3.0-or-later
/** Per-path Navigator visit counts + Quick Access ranking (AIA-74). */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCachedSettingsValues } from "./settingsHydrate.ts";

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

/** Settings toggle that turns visit recording off (default: ON). */
export const RECENT_TRACKING_SETTING_KEY = "core.trackRecentVisits";

export const visitStatsDoc = ref<VisitStatsDocument>({ visits: [] });

/**
 * Read the cached settings document to decide whether visits should be recorded.
 * Defaults to enabled until the setting is explicitly set to `false` (so a cold
 * start before settings hydrate still tracks, matching the on-by-default toggle).
 */
export function recentTrackingEnabled(): boolean {
  return getCachedSettingsValues()?.[RECENT_TRACKING_SETTING_KEY] !== false;
}

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
  if (!recentTrackingEnabled()) return null;
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

/** Remove a single recent-visit entry (hover close glyph in every Quick Access surface). */
export async function deleteVisit(path: string): Promise<void> {
  const trimmed = path.trim();
  if (!trimmed) return;
  try {
    visitStatsDoc.value = await invoke<VisitStatsDocument>("visit_stats_remove", {
      path: trimmed,
    });
  } catch {
    /* keep the in-memory list untouched on IPC failure */
  }
}

/** Forget all recorded visits (the "clear recent visits" affordance). */
export async function clearVisits(): Promise<void> {
  try {
    visitStatsDoc.value = await invoke<VisitStatsDocument>("visit_stats_clear");
  } catch {
    return;
  }
  // A cleared history must not stay debounced against a future re-record.
  lastRecordKey = "";
  lastRecordAt = 0;
}

/** Test helper — reset debounce + in-memory doc. */
export function resetVisitStatsForTests(): void {
  visitStatsDoc.value = { visits: [] };
  lastRecordKey = "";
  lastRecordAt = 0;
}
