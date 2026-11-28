// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator drill-through (ADR 0031): flatten the tree under the current
 * directory up to N levels of subfolders into one list.
 *
 * Two path concepts, deliberately kept apart:
 *  - **drill root** — the directory that was flattened. It stays put while the
 *    user walks inside the tree; the flattened rows do not change.
 *  - **current path** — the folder the breadcrumb/address bar shows. Walking
 *    inside the tree moves it (the tree overview stays).
 *
 * The on/off switch is session state per pane; the depth level is a persisted
 * user preference (`core.navigatorDrillDepth`).
 */

import { ref, shallowRef, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { NavigatorEntry } from "./navigatorBrowse.ts";
import { rememberSettingsDocument } from "./settingsHydrate.ts";
import { showHiddenItems } from "./showHidden/index.ts";
import { normalizePathKey } from "./pathKey.ts";
import { fsPathSeparator } from "./fsPath.ts";

/** Deepest level the UI may request (ADR 0031: 0..5, 0 = off). */
export const MAX_DRILL_DEPTH = 5;

/**
 * Rows the backend returns per request. The frontend raises this
 * progressively ("load more") — the row cap is request-driven, not a hard
 * truncation, so a big tree can be shown in full.
 */
export const DEFAULT_DRILL_LIMIT = 2_000;

/** Hard ceiling on rows one request may ask for. */
export const MAX_DRILL_LIMIT = 20_000;

/** Persisted default when the document has no drill depth yet. */
export const DEFAULT_DRILL_DEPTH = 2;

/**
 * Persisted preference: the depth level the user last chose (1..5).
 *
 * This is the *preferred* level for the next activation — the depth a pane
 * actually runs at lives in that pane's own state (`DrillPaneState.depth`),
 * because two panes can drill at different levels at the same time.
 */
export const drillDepth = ref(DEFAULT_DRILL_DEPTH);

export type DrillPane = "primary" | "secondary";

type DrillPaneState = {
  /** Drill-through is on for this pane. */
  enabled: Ref<boolean>;
  /** The flattened directory (the drill root). */
  root: Ref<string>;
  /**
   * The depth this pane is actually flattened at. Read by the badge, the
   * menu checkmark and the row source — the global `drillDepth` is only the
   * preferred level, so the two must not be confused.
   */
  depth: Ref<number>;
  /** Row cap the last request used ("load more" raises it). */
  limit: Ref<number>;
  /** Flattened rows. */
  entries: Ref<NavigatorEntry[]>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  /** A cap cut the walk short (rows are incomplete). */
  truncated: Ref<boolean>;
  /** The row cap cut it — a bigger limit would show more rows. */
  limitReached: Ref<boolean>;
  /** Filesystem entries the last walk visited. */
  visited: Ref<number>;
  /** How many subfolders contributed rows (shown as "from N subfolders"). */
  sourceDirCount: Ref<number>;
};

function createDrillPane(): DrillPaneState {
  return {
    enabled: ref(false),
    root: ref(""),
    depth: ref(0),
    limit: ref(DEFAULT_DRILL_LIMIT),
    entries: shallowRef<NavigatorEntry[]>([]),
    loading: ref(false),
    error: ref(null),
    truncated: ref(false),
    limitReached: ref(false),
    visited: ref(0),
    sourceDirCount: ref(0),
  };
}

export const primaryDrill = createDrillPane();
export const secondaryDrill = createDrillPane();

export function drillPaneState(pane: DrillPane): DrillPaneState {
  return pane === "secondary" ? secondaryDrill : primaryDrill;
}

type DrillEntryWire = {
  name: string;
  path: string;
  kind: string;
  isLink?: boolean | null;
  linkTarget?: string | null;
  size: number;
  modifiedMs?: number | null;
  relativePath: string;
  depth: number;
};

type DrillListingWire = {
  path: string;
  entries: DrillEntryWire[];
  dirCount: number;
  fileCount: number;
  sourceDirCount: number;
  truncated: boolean;
  limitReached?: boolean;
  visited?: number;
};

export function clampDrillDepth(depth: number): number {
  if (!Number.isFinite(depth)) return DEFAULT_DRILL_DEPTH;
  return Math.min(MAX_DRILL_DEPTH, Math.max(1, Math.trunc(depth)));
}

/** Sync apply from already-hydrated settings (cold-start critical path). */
export function applyNavigatorDrillDepthFromValues(
  values: Record<string, unknown>,
): void {
  const raw = values["core.navigatorDrillDepth"];
  const depth = typeof raw === "number" ? raw : DEFAULT_DRILL_DEPTH;
  drillDepth.value = clampDrillDepth(depth);
}

/** Persist the chosen level (1..5). The switch itself stays session-only. */
export function setDrillDepth(depth: number): void {
  const next = clampDrillDepth(depth);
  if (drillDepth.value === next) return;
  drillDepth.value = next;
  void invoke<{ version: number; values: Record<string, unknown> }>("settings_set", {
    patch: { "core.navigatorDrillDepth": next },
  })
    .then((doc) => rememberSettingsDocument(doc))
    .catch(() => {
      // Persist failed: keep the local value; the backend document wins next start.
    });
}

function toNavigatorEntry(entry: DrillEntryWire): NavigatorEntry {
  return {
    name: entry.name,
    path: entry.path,
    kind: entry.kind,
    size: entry.size,
    modifiedMs: entry.modifiedMs ?? null,
    providerKind: null,
    isLink: entry.isLink ?? null,
    linkTarget: entry.linkTarget ?? null,
    relativePath: entry.relativePath,
  };
}

/**
 * Fetch the flattened tree for `root` at `depth`.
 *
 * `limit` is the row cap for this request. It is request-driven on purpose:
 * the frontend raises it ("load more") instead of paging, because sorting
 * happens on the frontend — a paged backend result would break the global
 * sort order.
 */
export async function loadDrill(
  pane: DrillPane,
  root: string,
  depth: number = drillDepth.value,
  limit: number = DEFAULT_DRILL_LIMIT,
): Promise<void> {
  const state = drillPaneState(pane);
  const target = root.trim();
  if (!target || depth <= 0) {
    clearDrill(pane);
    return;
  }
  const rowLimit = Math.min(MAX_DRILL_LIMIT, Math.max(1, Math.trunc(limit)));
  state.loading.value = true;
  state.error.value = null;
  try {
    const result = await invoke<DrillListingWire>("navigator_drill_list", {
      path: target,
      depth,
      showHidden: showHiddenItems.value,
      limit: rowLimit,
    });
    state.entries.value = (result.entries ?? []).map(toNavigatorEntry);
    state.root.value = result.path || target;
    state.depth.value = depth;
    state.limit.value = rowLimit;
    state.truncated.value = Boolean(result.truncated);
    state.limitReached.value = Boolean(result.limitReached);
    state.visited.value = result.visited ?? 0;
    state.sourceDirCount.value = result.sourceDirCount ?? 0;
  } catch (err) {
    state.entries.value = [];
    state.error.value = err instanceof Error ? err.message : String(err);
  } finally {
    state.loading.value = false;
  }
}

/** Turn drill-through on for `pane`, rooted at the current directory. */
export async function enableDrill(
  pane: DrillPane,
  root: string,
  depth: number = drillDepth.value,
): Promise<void> {
  const state = drillPaneState(pane);
  state.enabled.value = true;
  state.root.value = root.trim();
  // A new depth is a new query — the previous row cap does not carry over.
  state.limit.value = DEFAULT_DRILL_LIMIT;
  await loadDrill(pane, root, depth, DEFAULT_DRILL_LIMIT);
}

/**
 * The next row cap for "load more": double it, capped at the hard ceiling.
 * Pure function so the gate can pin the growth curve.
 */
export function nextDrillLimit(current: number): number {
  if (!Number.isFinite(current) || current <= 0) return DEFAULT_DRILL_LIMIT;
  return Math.min(MAX_DRILL_LIMIT, Math.trunc(current) * 2);
}

/**
 * Show more flattened rows: widen the row cap and walk again.
 *
 * Not paging — the whole result set is returned and sorted on the frontend,
 * so "the first N rows" stays "the first N rows after sorting".
 */
export async function loadMoreDrill(pane: DrillPane): Promise<void> {
  const state = drillPaneState(pane);
  if (!state.enabled.value || !state.root.value.trim()) return;
  if (state.depth.value <= 0) return;
  if (state.limit.value >= MAX_DRILL_LIMIT) return;
  await loadDrill(
    pane,
    state.root.value,
    state.depth.value,
    nextDrillLimit(state.limit.value),
  );
}

/** Re-fetch the flattened tree (F5 / after a mutation). */
export async function refreshDrill(pane: DrillPane): Promise<void> {
  const state = drillPaneState(pane);
  if (!state.enabled.value) return;
  await loadDrill(pane, state.root.value, state.depth.value, state.limit.value);
}

/** Turn drill-through off and drop the flattened rows. */
export function clearDrill(pane: DrillPane): void {
  const state = drillPaneState(pane);
  state.enabled.value = false;
  state.root.value = "";
  state.depth.value = 0;
  state.limit.value = DEFAULT_DRILL_LIMIT;
  state.entries.value = [];
  state.error.value = null;
  state.truncated.value = false;
  state.limitReached.value = false;
  state.visited.value = 0;
  state.sourceDirCount.value = 0;
  state.loading.value = false;
}

/** True when `path` is the drill root itself or lives below it. */
export function isInsideDrillTree(path: string, root: string): boolean {
  const key = normalizePathKey(path);
  const rootKey = normalizePathKey(root);
  if (!key || !rootKey) return false;
  if (key === rootKey) return true;
  // macOS/POSIX paths use `/`; hard-coding `\` here made every in-tree
  // navigation look like it left the tree and auto-cleared the drill view.
  const separator = fsPathSeparator(rootKey);
  const prefix =
    rootKey.endsWith(separator) || rootKey.endsWith("\\") || rootKey.endsWith("/")
      ? rootKey
      : `${rootKey}${separator}`;
  return key.startsWith(prefix);
}

/**
 * Drill-through survives navigation while the user walks inside the tree and
 * turns off as soon as the pane leaves it.
 */
export function drillSurvivesNavigation(pane: DrillPane, path: string): boolean {
  const state = drillPaneState(pane);
  if (!state.enabled.value) return false;
  return isInsideDrillTree(path, state.root.value);
}

/**
 * Where write operations (new folder / new file / paste) land. Under
 * drill-through every write goes to the drill root — never into a nested
 * folder the user only sees through the flattened list.
 */
export function drillTargetPath(pane: DrillPane, currentPath: string): string {
  const state = drillPaneState(pane);
  if (state.enabled.value && state.root.value.trim()) return state.root.value;
  return currentPath;
}

/**
 * The pane a toolbar control acts on: the focused pane in dual-pane mode,
 * the primary pane otherwise. Pure function — keeps this module free of
 * imports from the dual-pane state (which imports the browse state).
 */
export function resolveDrillPane(
  activePane: "primary" | "secondary",
  dualPaneOn: boolean,
): DrillPane {
  return dualPaneOn && activePane === "secondary" ? "secondary" : "primary";
}

/** Root-relative location of a row (empty outside drill-through). */
export function entryRelativePath(entry: NavigatorEntry): string {
  return entry.relativePath ?? "";
}
