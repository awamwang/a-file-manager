// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  createEmptyOpenState,
  mergePinnedIntoOpenState,
  openStateWithoutPinned,
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
  [[tab("p1", "C:\\pin-a")], [tab("u1", "C:\\old")], [tab("p2", "C:\\pin-b")], [tab("u2", "C:\\old-2")]],
  2,
);
const restored = state(
  [[tab("t1", "D:\\one")], [tab("t2", "D:\\two")], [tab("t3", "D:\\three")]],
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
  state([[tab("p1", "C:\\pin-a")], [tab("t1", "D:\\one")]], 0),
  live,
  pinned,
);
assert.equal(duplicated.tabGroups.filter((group) => group[0]?.id === "p1").length, 1);

assert.deepEqual(openStateWithoutPinned(createEmptyOpenState(), pinned).tabGroups, []);

assert.equal(
  workGroupDisplayName({ name: "Default", isDefault: true }, "默认工作组"),
  "默认工作组",
);
assert.equal(
  workGroupDisplayName({ name: "Photos", isDefault: false }, "默认工作组"),
  "Photos",
);

console.log("tab-pin checks OK.");
