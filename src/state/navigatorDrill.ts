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

/** Deepest level the UI may request (ADR 0031: 0..5, 0 = off). */
export const MAX_DRILL_DEPTH = 5;

/** Persisted default when the document has no drill depth yet. */
export const DEFAULT_DRILL_DEPTH = 2;

/** Persisted preference: the depth level the user last chose (1..5). */
export const drillDepth = ref(DEFAULT_DRILL_DEPTH);

export type DrillPane = "primary" | "secondary";

type DrillPaneState = {
  /** Drill-through is on for this pane. */
  enabled: Ref<boolean>;
  /** The flattened directory (the drill root). */
  root: Ref<string>;
  /** Flattened rows. */
  entries: Ref<NavigatorEntry[]>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  /** A cap cut the walk short (rows are incomplete). */
  truncated: Ref<boolean>;
  /** How many subfolders contributed rows (shown as "from N subfolders"). */
  sourceDirCount: Ref<number>;
};

function createDrillPane(): DrillPaneState {
  return {
    enabled: ref(false),
    root: ref(""),
    entries: shallowRef<NavigatorEntry[]>([]),
    loading: ref(false),
    error: ref(null),
    truncated: ref(false),
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

/** Fetch the flattened tree for `root` at `depth`. */
export async function loadDrill(
  pane: DrillPane,
  root: string,
  depth: number = drillDepth.value,
): Promise<void> {
  const state = drillPaneState(pane);
  const target = root.trim();
  if (!target || depth <= 0) {
    clearDrill(pane);
    return;
  }
  state.loading.value = true;
  state.error.value = null;
  try {
    const result = await invoke<DrillListingWire>("navigator_drill_list", {
      path: target,
      depth,
      showHidden: showHiddenItems.value,
    });
    state.entries.value = (result.entries ?? []).map(toNavigatorEntry);
    state.root.value = result.path || target;
    state.truncated.value = Boolean(result.truncated);
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
  await loadDrill(pane, root, depth);
}

/** Re-fetch the flattened tree (F5 / after a mutation). */
export async function refreshDrill(pane: DrillPane): Promise<void> {
  const state = drillPaneState(pane);
  if (!state.enabled.value) return;
  await loadDrill(pane, state.root.value, drillDepth.value);
}

/** Turn drill-through off and drop the flattened rows. */
export function clearDrill(pane: DrillPane): void {
  const state = drillPaneState(pane);
  state.enabled.value = false;
  state.root.value = "";
  state.entries.value = [];
  state.error.value = null;
  state.truncated.value = false;
  state.sourceDirCount.value = 0;
  state.loading.value = false;
}

/** True when `path` is the drill root itself or lives below it. */
export function isInsideDrillTree(path: string, root: string): boolean {
  const key = normalizePathKey(path);
  const rootKey = normalizePathKey(root);
  if (!key || !rootKey) return false;
  if (key === rootKey) return true;
  const prefix = rootKey.endsWith("\\") || rootKey.endsWith("/") ? rootKey : `${rootKey}\\`;
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
