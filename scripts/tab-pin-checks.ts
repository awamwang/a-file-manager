// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  clampOpenStateIndices,
  createEmptyOpenState,
  mergePinnedIntoOpenState,
  openStateWithoutPinned,
  safeOpenStateIndex,
  type WorkGroupOpenState,
  type WorkGroupOpenStateTab,
} from "../src/state/openStateModel.ts";
import { workGroupDisplayName } from "../src/state/workGroupName.ts";

function tab(id: string, path: string): WorkGroupOpenStateTab {
  return { id, name: id, path, type: "directory" };
}

function state(groups: WorkGroupOpenStateTab[][], current = 0): WorkGroupOpenState {
  return {
    tabGroups: groups,
    currentTabGroupIndex: current,
    currentTabIndex: 0,
  };
}

const pinned = new Set(["p1", "p2"]);
const live = state(
  [[tab("p1", "C:\\Temp\\afm-fixture\\pin-a")], [tab("u1", "C:\\Temp\\afm-fixture\\old")], [tab("p2", "C:\\Temp\\afm-fixture\\pin-b")], [tab("u2", "C:\\Temp\\afm-fixture\\old-2")]],
  2,
);
const restored = state(
  [[tab("t1", "D:\\Temp\\afm-fixture\\one")], [tab("t2", "D:\\Temp\\afm-fixture\\two")], [tab("t3", "D:\\Temp\\afm-fixture\\three")]],
  1,
);

const merged = mergePinnedIntoOpenState(restored, live, pinned);
assert.deepEqual(
  merged.tabGroups.map((group) => group[0]?.id),
  ["p1", "t1", "p2", "t2", "t3"],
);
assert.equal(merged.currentTabGroupIndex, 2);

const saved = openStateWithoutPinned(live, pinned);
assert.deepEqual(
  saved.tabGroups.map((group) => group[0]?.id),
  ["u1", "u2"],
);

const untouched = mergePinnedIntoOpenState(restored, live, new Set());
assert.equal(untouched.tabGroups.length, 3);
assert.equal(untouched.currentTabGroupIndex, 1);

const duplicated = mergePinnedIntoOpenState(
  state([[tab("p1", "C:\\Temp\\afm-fixture\\pin-a")], [tab("t1", "D:\\Temp\\afm-fixture\\one")]], 0),
  live,
  pinned,
);
assert.equal(duplicated.tabGroups.filter((group) => group[0]?.id === "p1").length, 1);

assert.deepEqual(openStateWithoutPinned(createEmptyOpenState(), pinned).tabGroups, []);

// AIA-228: pinning the active (or earlier) tab must not emit a negative index —
// Rust deserializes currentTabGroupIndex as usize and rejects -1, blocking WG switch.
const activePinned = openStateWithoutPinned(
  state(
    [[tab("p1", "C:\\Temp\\afm-fixture\\pin-a")], [tab("u1", "C:\\Temp\\afm-fixture\\keep")]],
    0,
  ),
  new Set(["p1"]),
);
assert.equal(activePinned.currentTabGroupIndex, 0);
assert.ok(activePinned.currentTabGroupIndex >= 0);
assert.deepEqual(
  activePinned.tabGroups.map((group) => group[0]?.id),
  ["u1"],
);

const earlierPinned = openStateWithoutPinned(
  state(
    [
      [tab("p1", "C:\\Temp\\afm-fixture\\pin-a")],
      [tab("u1", "C:\\Temp\\afm-fixture\\keep")],
      [tab("u2", "C:\\Temp\\afm-fixture\\keep-2")],
    ],
    2,
  ),
  new Set(["p1"]),
);
assert.equal(earlierPinned.currentTabGroupIndex, 1);
assert.deepEqual(
  earlierPinned.tabGroups.map((group) => group[0]?.id),
  ["u1", "u2"],
);

const onlyPinned = openStateWithoutPinned(
  state([[tab("p1", "C:\\Temp\\afm-fixture\\pin-a")]], 0),
  new Set(["p1"]),
);
assert.deepEqual(onlyPinned.tabGroups, []);
assert.equal(onlyPinned.currentTabGroupIndex, 0);

const switchWithActivePin = mergePinnedIntoOpenState(
  state([[tab("t1", "D:\\Temp\\afm-fixture\\one")]], 0),
  state(
    [[tab("p1", "C:\\Temp\\afm-fixture\\pin-a")], [tab("u1", "C:\\Temp\\afm-fixture\\old")]],
    0,
  ),
  new Set(["p1"]),
);
assert.ok(switchWithActivePin.currentTabGroupIndex >= 0);
assert.deepEqual(
  switchWithActivePin.tabGroups.map((group) => group[0]?.id),
  ["p1", "t1"],
);

assert.equal(
  workGroupDisplayName({ name: "Default", isDefault: true }, "默认工作组"),
  "默认工作组",
);
assert.equal(
  workGroupDisplayName({ name: "Photos", isDefault: false }, "默认工作组"),
  "Photos",
);

// AIA-245: non-finite indices must not survive clamp (JSON would emit null → Rust usize fail).
assert.equal(safeOpenStateIndex(Number.NaN), 0);
assert.equal(safeOpenStateIndex(Number.POSITIVE_INFINITY), 0);
assert.equal(safeOpenStateIndex(-3.7), 0);
assert.equal(safeOpenStateIndex(2.9), 2);
assert.equal(safeOpenStateIndex(null), 0);
assert.equal(safeOpenStateIndex(undefined), 0);

const nanClamped = clampOpenStateIndices(
  state([[tab("u1", "C:\\Temp\\afm-fixture\\keep")]], Number.NaN),
);
assert.equal(nanClamped.currentTabGroupIndex, 0);
assert.equal(JSON.parse(JSON.stringify(nanClamped)).currentTabGroupIndex, 0);

const leavingAfterActivePin = openStateWithoutPinned(
  state(
    [
      [tab("p1", "C:\\Temp\\afm-fixture\\pin-a")],
      [tab("u1", "C:\\Temp\\afm-fixture\\keep")],
    ],
    0,
  ),
  new Set(["p1"]),
);
const ipcPayload = JSON.parse(JSON.stringify(leavingAfterActivePin));
assert.equal(typeof ipcPayload.currentTabGroupIndex, "number");
assert.ok(ipcPayload.currentTabGroupIndex >= 0);
assert.notEqual(ipcPayload.currentTabGroupIndex, null);

console.log("tab-pin checks OK.");
