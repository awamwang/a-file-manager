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
  openStateAfterReorderGroups,
  openStateAfterSwitchAdjacent,
  openStateAfterSwitchGroup,
  normalizePathKey,
  openStateEnsureMemberPaths,
  openStateFromPaths,
  openStateOpenOrFocusPath,
  openStateWithNewTab,
  stripTabsFromOpenState,
  toNativeFsPath,
  type WorkGroupOpenState,
} from "../src/state/openState/model.ts";

assert.equal(toNativeFsPath("G:/Projects/A/demo-app"), "G:\\Projects\\A\\demo-app");
assert.equal(toNativeFsPath("afm://guide"), "afm://guide");
assert.equal(displayNameFromPath("G:/Projects/A/demo-app"), "demo-app");
assert.equal(displayNameFromPath("C:\\Temp\\afm-fixture\\home\\"), "home");
assert.equal(normalizePathKey("  G:\\Projects\\A\\  "), normalizePathKey("g:/projects/a"));
assert.equal(normalizePathKey("G:/Projects/A"), "g:\\projects\\a");

const empty = createEmptyOpenState();
assert.deepEqual(empty.tabGroups, []);
assert.equal(stripTabsFromOpenState(empty).length, 0);

let state = openStateFromPaths([
  "G:/Projects/A/demo-app-public",
  "G:/Projects/A/demo-app",
  "G:/Projects/A/tools/cli-tool",
]);
assert.equal(state.tabGroups.length, 3);
assert.equal(state.currentTabGroupIndex, 0);
assert.equal(activeStripTab(state)?.path, "G:\\Projects\\A\\demo-app-public");

state = openStateAfterSwitchGroup(state, 2);
assert.equal(state.currentTabGroupIndex, 2);
assert.equal(activeStripTab(state)?.name, "cli-tool");

state = openStateAfterNavigate(state, "G:/Projects/A/tools/cli-tool/src");
assert.equal(activeStripTab(state)?.path, "G:\\Projects\\A\\tools\\cli-tool\\src");
assert.equal(activeStripTab(state)?.name, "src");
assert.equal(state.tabGroups[0][0].path, "G:\\Projects\\A\\demo-app-public");

state = openStateWithNewTab(state, "G:/Projects/Workspace");
assert.equal(state.tabGroups.length, 4);
assert.equal(state.currentTabGroupIndex, 3);
assert.equal(activeStripTab(state)?.name, "Workspace");

state = openStateAfterCloseGroup(state, 1);
assert.equal(state.tabGroups.length, 3);
assert.ok(state.tabGroups.every((g) => g[0].path !== "G:\\Projects\\A\\demo-app"));

// Closing the active group selects a neighbor
state = openStateAfterSwitchGroup(state, 1);
const closedActive = openStateAfterCloseGroup(state, 1);
assert.equal(closedActive.tabGroups.length, 2);
assert.ok(closedActive.currentTabGroupIndex < closedActive.tabGroups.length);

// Sigma-shaped restore (one pane per group + currentTabGroupIndex)
const sigmaLike: WorkGroupOpenState = {
  tabGroups: [
    [{ id: "a", name: "skills-kit", path: "G:/Projects/Org/skills-kit", type: "directory" }],
    [{ id: "b", name: "Workspace", path: "G:/Projects/Workspace", type: "directory" }],
    [{ id: "c", name: "skills", path: "G:/Projects/Org/skills", type: "directory" }],
  ],
  currentTabGroupIndex: 2,
  currentTabIndex: 0,
};
const restored = normalizeRestoredOpenState(sigmaLike);
assert.equal(stripTabsFromOpenState(restored).length, 3);
assert.equal(activeStripTab(restored)?.path, "G:\\Projects\\Org\\skills");

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

// Strip drag-reorder: move last group to front; active index follows moved tab.
let reorder = openStateFromPaths([
  "G:/Projects/A",
  "G:/Projects/B",
  "G:/Projects/C",
]);
reorder = openStateAfterSwitchGroup(reorder, 2);
reorder = openStateAfterReorderGroups(reorder, 2, 0);
assert.equal(reorder.tabGroups[0][0].path, "G:\\Projects\\C");
assert.equal(reorder.currentTabGroupIndex, 0);
assert.equal(activeStripTab(reorder)?.name, "C");
reorder = openStateAfterReorderGroups(reorder, 0, 2);
assert.equal(reorder.tabGroups[2][0].path, "G:\\Projects\\C");
assert.equal(reorder.currentTabGroupIndex, 2);
assert.equal(reorder.tabGroups.length, 3);
// Out-of-range / non-integer must not grow or insert holes.
const guarded = openStateAfterReorderGroups(reorder, 0, 99);
assert.equal(guarded.tabGroups.length, 3);
const same = openStateAfterReorderGroups(reorder, 1, 1);
assert.equal(same.tabGroups.length, 3);

assert.equal(findTabGroupIndexByPath(restored, "G:\\Projects\\Workspace"), 1);
assert.equal(findTabGroupIndexByPath(restored, "G:/missing"), -1);

const focusHit = openStateOpenOrFocusPath(restored, "G:/Projects/Workspace");
assert.equal(focusHit.focusedExisting, true);
assert.equal(focusHit.groupIndex, 1);
assert.equal(focusHit.state.currentTabGroupIndex, 1);
assert.equal(focusHit.state.tabGroups.length, 3);

const focusMiss = openStateOpenOrFocusPath(restored, "G:/Projects/NewFolder");
assert.equal(focusMiss.focusedExisting, false);
assert.equal(focusMiss.state.tabGroups.length, 4);
assert.equal(focusMiss.state.currentTabGroupIndex, 3);
assert.equal(activeStripTab(focusMiss.state)?.name, "NewFolder");
assert.equal(activeStripTab(focusMiss.state)?.path, "G:\\Projects\\NewFolder");

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
assert.deepEqual(soft.opened, ["G:\\Projects\\B", "G:\\Projects\\C"]);
assert.equal(soft.state.tabGroups.length, 4);
assert.equal(soft.state.currentTabGroupIndex, 1);
assert.equal(activeStripTab(soft.state)?.path, "G:\\Projects\\Extra");

const softNone = openStateEnsureMemberPaths(soft.state, ["G:/Projects/A", "G:/Projects/B"]);
assert.equal(softNone.opened.length, 0);
assert.equal(softNone.state.tabGroups.length, 4);

console.log("open-state-checks: ok");
