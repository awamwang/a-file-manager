// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure Work Group Open State helpers (Navigator tab strip ↔ S6 schema). */

export type WorkGroupOpenStateTab = {
  id: string;
  name: string;
  path: string;
  type: string;
};

export type WorkGroupOpenState = {
  tabGroups: WorkGroupOpenStateTab[][];
  currentTabGroupIndex: number;
  currentTabIndex: number;
};

export type StripTab = {
  id: string;
  name: string;
  path: string;
  groupIndex: number;
};

let tabSeq = 0;

export function nextTabId(prefix = "tab"): string {
  tabSeq += 1;
  return `${prefix}-${Date.now()}-${tabSeq}`;
}

export function displayNameFromPath(path: string): string {
  const cleaned = path.replace(/\\/g, "/").replace(/\/+$/, "");
  const parts = cleaned.split("/");
  return parts[parts.length - 1] || path || "Tab";
}

export function createEmptyOpenState(): WorkGroupOpenState {
  return {
    tabGroups: [],
    currentTabGroupIndex: 0,
    currentTabIndex: 0,
  };
}

export function createDirectoryTab(path: string, id?: string): WorkGroupOpenStateTab {
  const trimmed = path.trim();
  return {
    id: id ?? nextTabId(),
    name: displayNameFromPath(trimmed),
    path: trimmed,
    type: "directory",
  };
}

export function cloneOpenState(state: WorkGroupOpenState): WorkGroupOpenState {
  return {
    tabGroups: state.tabGroups.map((group) => group.map((tab) => ({ ...tab }))),
    currentTabGroupIndex: state.currentTabGroupIndex,
    currentTabIndex: state.currentTabIndex,
  };
}

export function clampOpenStateIndices(state: WorkGroupOpenState): WorkGroupOpenState {
  const next = cloneOpenState(state);
  if (next.tabGroups.length === 0) {
    next.currentTabGroupIndex = 0;
    next.currentTabIndex = 0;
    return next;
  }
  next.currentTabGroupIndex = Math.min(
    next.currentTabGroupIndex,
    next.tabGroups.length - 1,
  );
  const group = next.tabGroups[next.currentTabGroupIndex] ?? [];
  next.currentTabIndex =
    group.length === 0 ? 0 : Math.min(next.currentTabIndex, group.length - 1);
  return next;
}

/** One strip tab per tab group (Day-1: no split panes in the strip). */
export function stripTabsFromOpenState(state: WorkGroupOpenState): StripTab[] {
  return state.tabGroups.map((group, groupIndex) => {
    const paneIndex =
      groupIndex === state.currentTabGroupIndex
        ? Math.min(state.currentTabIndex, Math.max(group.length - 1, 0))
        : 0;
    const tab = group[paneIndex] ?? group[0];
    return {
      id: tab?.id ?? `group-${groupIndex}`,
      name: tab?.name || displayNameFromPath(tab?.path ?? ""),
      path: tab?.path ?? "",
      groupIndex,
    };
  });
}

export function activeStripTab(state: WorkGroupOpenState): StripTab | null {
  const tabs = stripTabsFromOpenState(state);
  if (!tabs.length) return null;
  const index = Math.min(state.currentTabGroupIndex, tabs.length - 1);
  return tabs[index] ?? null;
}

export function openStateWithNewTab(
  state: WorkGroupOpenState,
  path: string,
): WorkGroupOpenState {
  const next = cloneOpenState(state);
  next.tabGroups.push([createDirectoryTab(path)]);
  next.currentTabGroupIndex = next.tabGroups.length - 1;
  next.currentTabIndex = 0;
  return next;
}

export function openStateAfterCloseGroup(
  state: WorkGroupOpenState,
  groupIndex: number,
): WorkGroupOpenState {
  const next = cloneOpenState(state);
  if (groupIndex < 0 || groupIndex >= next.tabGroups.length) {
    return clampOpenStateIndices(next);
  }
  next.tabGroups.splice(groupIndex, 1);
  if (next.tabGroups.length === 0) {
    next.currentTabGroupIndex = 0;
    next.currentTabIndex = 0;
    return next;
  }
  if (groupIndex < next.currentTabGroupIndex) {
    next.currentTabGroupIndex -= 1;
  } else if (groupIndex === next.currentTabGroupIndex) {
    next.currentTabGroupIndex = Math.min(groupIndex, next.tabGroups.length - 1);
  }
  next.currentTabIndex = 0;
  return clampOpenStateIndices(next);
}

export function openStateAfterSwitchGroup(
  state: WorkGroupOpenState,
  groupIndex: number,
): WorkGroupOpenState {
  const next = cloneOpenState(state);
  if (groupIndex < 0 || groupIndex >= next.tabGroups.length) {
    return clampOpenStateIndices(next);
  }
  next.currentTabGroupIndex = groupIndex;
  // Preserve currentTabIndex like Sigma; clamp bounds it to the target group.
  return clampOpenStateIndices(next);
}

/** Case-insensitive path key for tab-group identity (Sigma openOrFocus / dedupe). */
export function normalizePathKey(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

/** Index of the first tab group that already shows `path`, or -1. */
export function findTabGroupIndexByPath(
  state: WorkGroupOpenState,
  path: string,
): number {
  const key = normalizePathKey(path);
  if (!key) return -1;
  return state.tabGroups.findIndex((group) =>
    group.some((tab) => normalizePathKey(tab.path ?? "") === key),
  );
}

export type SwitchAdjacentDirection = "next" | "previous";

/** Cycle `currentTabGroupIndex` (wrap). No-op when fewer than 2 groups. */
export function openStateAfterSwitchAdjacent(
  state: WorkGroupOpenState,
  direction: SwitchAdjacentDirection,
): WorkGroupOpenState {
  const total = state.tabGroups.length;
  if (total < 2) return clampOpenStateIndices(state);
  const current = Math.min(Math.max(state.currentTabGroupIndex, 0), total - 1);
  const offset = direction === "next" ? 1 : -1;
  const nextIndex = (current + offset + total) % total;
  return openStateAfterSwitchGroup(state, nextIndex);
}

export type OpenOrFocusResult = {
  state: WorkGroupOpenState;
  /** True when an existing page group was selected; false when a new group was appended. */
  focusedExisting: boolean;
  groupIndex: number;
};

/**
 * Select an existing page group (tab group) that already shows `path`, or append a new group.
 * Mirrors Sigma `workspaces.openOrFocusTabGroup`.
 */
export function openStateOpenOrFocusPath(
  state: WorkGroupOpenState,
  path: string,
): OpenOrFocusResult {
  const trimmed = path.trim();
  const existing = findTabGroupIndexByPath(state, trimmed);
  if (existing >= 0) {
    return {
      state: openStateAfterSwitchGroup(state, existing),
      focusedExisting: true,
      groupIndex: existing,
    };
  }
  const next = openStateWithNewTab(state, trimmed);
  return {
    state: next,
    focusedExisting: false,
    groupIndex: next.currentTabGroupIndex,
  };
}

/** Update the active pane path/name after address-bar / folder navigation. */
export function openStateAfterNavigate(
  state: WorkGroupOpenState,
  path: string,
): WorkGroupOpenState {
  const trimmed = path.trim();
  if (!trimmed) return clampOpenStateIndices(state);

  if (state.tabGroups.length === 0) {
    return openStateWithNewTab(state, trimmed);
  }

  const next = clampOpenStateIndices(cloneOpenState(state));
  const group = next.tabGroups[next.currentTabGroupIndex];
  if (!group || group.length === 0) {
    next.tabGroups[next.currentTabGroupIndex] = [createDirectoryTab(trimmed)];
    next.currentTabIndex = 0;
    return next;
  }
  const pane = group[next.currentTabIndex] ?? group[0];
  pane.path = trimmed;
  pane.name = displayNameFromPath(trimmed);
  pane.type = "directory";
  return next;
}

/** Replace strip with one tab-group per path (Projects open-all → tabs only). */
export function openStateFromPaths(paths: string[]): WorkGroupOpenState {
  const cleaned = paths.map((p) => p.trim()).filter(Boolean);
  if (!cleaned.length) return createEmptyOpenState();
  return {
    tabGroups: cleaned.map((path) => [createDirectoryTab(path)]),
    currentTabGroupIndex: 0,
    currentTabIndex: 0,
  };
}

export type EnsureMemberTabsResult = {
  state: WorkGroupOpenState;
  /** Member paths that were newly appended as tab groups. */
  opened: string[];
};

/**
 * Hard members → runtime Open State (Sigma soft sync):
 * append a directory tab group for each missing member path; never close extras.
 */
export function openStateEnsureMemberPaths(
  state: WorkGroupOpenState,
  memberPaths: string[],
): EnsureMemberTabsResult {
  const next = cloneOpenState(state);
  const opened: string[] = [];
  const seenKeys = new Set<string>();

  for (const raw of memberPaths) {
    const trimmed = raw.trim();
    if (!trimmed) continue;
    const key = normalizePathKey(trimmed);
    if (!key || seenKeys.has(key)) continue;
    seenKeys.add(key);
    if (findTabGroupIndexByPath(next, trimmed) >= 0) continue;
    next.tabGroups.push([createDirectoryTab(trimmed)]);
    opened.push(trimmed);
  }

  return { state: clampOpenStateIndices(next), opened };
}

export function normalizeRestoredOpenState(
  state: WorkGroupOpenState | null | undefined,
): WorkGroupOpenState {
  if (!state) return createEmptyOpenState();
  return clampOpenStateIndices({
    tabGroups: (state.tabGroups ?? []).map((group) =>
      (group ?? []).map((tab) => ({
        id: tab.id || nextTabId(),
        name: tab.name || displayNameFromPath(tab.path ?? ""),
        path: tab.path ?? "",
        type: tab.type || "directory",
      })),
    ),
    currentTabGroupIndex: state.currentTabGroupIndex ?? 0,
    currentTabIndex: state.currentTabIndex ?? 0,
  });
}
