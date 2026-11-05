// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/open-state-checks.ts

import assert from "node:assert/strict";
import {
  activeStripTab,
  createEmptyOpenState,
  displayNameFromPath,
  findTabGroupIndexByPath,
  normalizeRestoredOpenState,
  openStateAfterCloseGroup,
  openStateAfterNavigate,
  openStateAfterSwitchAdjacent,
  openStateAfterSwitchGroup,
  normalizePathKey,
  openStateEnsureMemberPaths,
  openStateFromPaths,
  openStateOpenOrFocusPath,
  openStateWithNewTab,
  stripTabsFromOpenState,
  type WorkGroupOpenState,
} from "../src/state/openStateModel.ts";

assert.equal(displayNameFromPath("G:/Projects/Stock/vibe-astock"), "vibe-astock");
assert.equal(displayNameFromPath("C:\\Users\\Administrator\\"), "Administrator");
assert.equal(normalizePathKey("  G:\\Projects\\A\\  "), normalizePathKey("g:/projects/a"));

const empty = createEmptyOpenState();
assert.deepEqual(empty.tabGroups, []);
assert.equal(stripTabsFromOpenState(empty).length, 0);

let state = openStateFromPaths([
  "G:/Projects/Stock/vibe-astock-public",
  "G:/Projects/Stock/vibe-astock",
  "G:/Projects/Stock/tdx/tdx-mcp",
]);
assert.equal(state.tabGroups.length, 3);
assert.equal(state.currentTabGroupIndex, 0);
assert.equal(activeStripTab(state)?.path, "G:/Projects/Stock/vibe-astock-public");

state = openStateAfterSwitchGroup(state, 2);
assert.equal(state.currentTabGroupIndex, 2);
assert.equal(activeStripTab(state)?.name, "tdx-mcp");

state = openStateAfterNavigate(state, "G:/Projects/Stock/tdx/tdx-mcp/src");
assert.equal(activeStripTab(state)?.path, "G:/Projects/Stock/tdx/tdx-mcp/src");
assert.equal(activeStripTab(state)?.name, "src");
assert.equal(state.tabGroups[0][0].path, "G:/Projects/Stock/vibe-astock-public");

state = openStateWithNewTab(state, "G:/Projects/Tools");
assert.equal(state.tabGroups.length, 4);
assert.equal(state.currentTabGroupIndex, 3);
assert.equal(activeStripTab(state)?.name, "Tools");

state = openStateAfterCloseGroup(state, 1);
assert.equal(state.tabGroups.length, 3);
assert.ok(state.tabGroups.every((g) => g[0].path !== "G:/Projects/Stock/vibe-astock"));

// Closing the active group selects a neighbor
state = openStateAfterSwitchGroup(state, 1);
const closedActive = openStateAfterCloseGroup(state, 1);
assert.equal(closedActive.tabGroups.length, 2);
assert.ok(closedActive.currentTabGroupIndex < closedActive.tabGroups.length);

// Sigma-shaped restore (one pane per group + currentTabGroupIndex)
const sigmaLike: WorkGroupOpenState = {
  tabGroups: [
    [{ id: "a", name: "awam-skills", path: "G:/Projects/19AI/awam-skills", type: "directory" }],
    [{ id: "b", name: "Tools", path: "G:/Projects/Tools", type: "directory" }],
    [{ id: "c", name: "skills", path: "G:/Projects/19AI/skills", type: "directory" }],
  ],
  currentTabGroupIndex: 2,
  currentTabIndex: 0,
};
const restored = normalizeRestoredOpenState(sigmaLike);
assert.equal(stripTabsFromOpenState(restored).length, 3);
assert.equal(activeStripTab(restored)?.path, "G:/Projects/19AI/skills");

// AIA-77: adjacent page-group cycle (wrap) + openOrFocus select
let cycle = openStateFromPaths([
  "G:/Projects/A",
  "G:/Projects/B",
  "G:/Projects/C",
]);
cycle = openStateAfterSwitchAdjacent(cycle, "next");
assert.equal(cycle.currentTabGroupIndex, 1);
assert.equal(activeStripTab(cycle)?.name, "B");
cycle = openStateAfterSwitchAdjacent(cycle, "previous");
assert.equal(cycle.currentTabGroupIndex, 0);
cycle = openStateAfterSwitchAdjacent(cycle, "previous");
assert.equal(cycle.currentTabGroupIndex, 2);
assert.equal(activeStripTab(cycle)?.name, "C");
const single = openStateFromPaths(["G:/only"]);
assert.equal(openStateAfterSwitchAdjacent(single, "next").currentTabGroupIndex, 0);

assert.equal(findTabGroupIndexByPath(restored, "G:\\Projects\\Tools"), 1);
assert.equal(findTabGroupIndexByPath(restored, "G:/missing"), -1);

const focusHit = openStateOpenOrFocusPath(restored, "G:/Projects/Tools");
assert.equal(focusHit.focusedExisting, true);
assert.equal(focusHit.groupIndex, 1);
assert.equal(focusHit.state.currentTabGroupIndex, 1);
assert.equal(focusHit.state.tabGroups.length, 3);

const focusMiss = openStateOpenOrFocusPath(restored, "G:/Projects/NewFolder");
assert.equal(focusMiss.focusedExisting, false);
assert.equal(focusMiss.state.tabGroups.length, 4);
assert.equal(focusMiss.state.currentTabGroupIndex, 3);
assert.equal(activeStripTab(focusMiss.state)?.name, "NewFolder");

// Soft sync: append missing members only; keep extras; do not jump focus.
const softBase = openStateFromPaths(["G:/Projects/A", "G:/Projects/Extra"]);
softBase.currentTabGroupIndex = 1;
const soft = openStateEnsureMemberPaths(softBase, [
  "G:/Projects/A",
  "G:\\Projects\\B",
  "g:/projects/b",
  "G:/Projects/C",
]);
assert.equal(soft.opened.length, 2);
assert.deepEqual(soft.opened, ["G:\\Projects\\B", "G:/Projects/C"]);
assert.equal(soft.state.tabGroups.length, 4);
assert.equal(soft.state.currentTabGroupIndex, 1);
assert.equal(activeStripTab(soft.state)?.path, "G:/Projects/Extra");

const softNone = openStateEnsureMemberPaths(soft.state, ["G:/Projects/A", "G:/Projects/B"]);
assert.equal(softNone.opened.length, 0);
assert.equal(softNone.state.tabGroups.length, 4);

console.log("open-state-checks: ok");
