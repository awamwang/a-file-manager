// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  arrowDirectionFromKey,
  estimateIconColumns,
  moveSelectionByArrow,
  nextIndexForArrow,
} from "../src/state/navigatorKeyboardNav.ts";
import {
  FOCUS_CYCLE_REGIONS,
  nextFocusRegion,
} from "../src/state/focusCycle.ts";
import { BUILTIN_SHORTCUTS, resolveDispatchMap } from "../src/shortcuts/registry.ts";
import { chordKey } from "../src/shortcuts/chord.ts";

assert.equal(nextIndexForArrow(-1, "down", 5, 1), 0);
assert.equal(nextIndexForArrow(-1, "up", 5, 1), 4);
assert.equal(nextIndexForArrow(2, "down", 5, 1), 3);
assert.equal(nextIndexForArrow(0, "up", 5, 1), 0);
assert.equal(nextIndexForArrow(1, "right", 6, 3), 2);
assert.equal(nextIndexForArrow(1, "left", 6, 3), 0);
assert.equal(nextIndexForArrow(1, "down", 6, 3), 4);
assert.equal(nextIndexForArrow(4, "up", 6, 3), 1);
assert.equal(nextIndexForArrow(5, "down", 6, 3), 5);

assert.equal(estimateIconColumns(400, "iconsMedium"), 4);
assert.equal(estimateIconColumns(50, "iconsLarge"), 1);
assert.equal(estimateIconColumns(400, "details"), 1);

assert.equal(arrowDirectionFromKey("ArrowDown"), "down");
assert.equal(arrowDirectionFromKey("a"), null);

const paths = ["a", "b", "c", "d"];
const first = moveSelectionByArrow(paths, "down", 1, false, {
  selectedPaths: [],
  selectionAnchor: null,
});
assert.equal(first?.path, "a");
assert.deepEqual(first?.state.selectedPaths, ["a"]);

const second = moveSelectionByArrow(paths, "down", 1, false, first!.state);
assert.equal(second?.path, "b");

const extended = moveSelectionByArrow(paths, "down", 1, true, second!.state);
assert.equal(extended?.path, "c");
assert.deepEqual(extended?.state.selectedPaths, ["b", "c"]);
assert.equal(extended?.state.selectionAnchor, "b");

const grid = moveSelectionByArrow(
  paths,
  "right",
  2,
  false,
  { selectedPaths: ["a"], selectionAnchor: "a" },
);
assert.equal(grid?.path, "b");

assert.deepEqual(FOCUS_CYCLE_REGIONS, ["tabstrip", "sidenav", "content", "statusbar"]);
assert.equal(nextFocusRegion(null), "tabstrip");
assert.equal(nextFocusRegion("tabstrip"), "sidenav");
assert.equal(nextFocusRegion("statusbar"), "tabstrip");
assert.equal(nextFocusRegion("tabstrip", true), "statusbar");
assert.equal(nextFocusRegion(null, true), "statusbar");

const winners = resolveDispatchMap(BUILTIN_SHORTCUTS);
assert.equal(winners.get(chordKey({ key: "f6" }))?.id, "chrome.focusCycle");
assert.equal(
  winners.get(chordKey({ shift: true, key: "f6" }))?.id,
  "chrome.focusCycleReverse",
);

console.log("keyboard-nav checks OK (AIA-190).");
