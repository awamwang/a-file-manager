// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  clampOpenStateIndices,
  collapsePinnedIds,
  createEmptyOpenState,
  ensureUniqueTabIds,
  mergePinnedIntoOpenState,
  normalizeRestoredOpenState,
  openStateWithoutPinned,
  safeOpenStateIndex,
  type WorkGroupOpenState,
  type WorkGroupOpenStateTab,
} from "../src/state/openState/model.ts";
import {
  commitPinnedPersistence,
  ensureSinglePinnedTab,
  hydratePersistedPins,
  isTabPinned,
  pickSinglePinnedId,
  pinnedTabIds,
  readPinnedSnapshots,
  togglePinnedTab,
  writePinnedSnapshots,
} from "../src/state/tabPins.ts";
import { workGroupDisplayName } from "../src/state/workGroup/name.ts";

const memoryStore = new Map<string, string>();
const localStorageStub = {
  getItem(key: string) {
    return memoryStore.has(key) ? memoryStore.get(key)! : null;
  },
  setItem(key: string, value: string) {
    memoryStore.set(key, String(value));
  },
  removeItem(key: string) {
    memoryStore.delete(key);
  },
};
Object.defineProperty(globalThis, "localStorage", {
  value: localStorageStub,
  configurable: true,
});
memoryStore.clear();

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

// AIA-260: pin toggle is exclusive — pinning B replaces A.
pinnedTabIds.value = [];
togglePinnedTab("a");
assert.deepEqual(pinnedTabIds.value, ["a"]);
togglePinnedTab("b");
assert.deepEqual(pinnedTabIds.value, ["b"]);
assert.equal(isTabPinned("a"), false);
assert.equal(isTabPinned("b"), true);
togglePinnedTab("b");
assert.deepEqual(pinnedTabIds.value, []);

assert.equal(pickSinglePinnedId(["p1", "p2"], "p2"), "p2");
assert.equal(pickSinglePinnedId(["p1", "p2"], "missing"), "p1");
assert.equal(pickSinglePinnedId([], "p1"), null);

pinnedTabIds.value = ["p1", "p2", "p3"];
ensureSinglePinnedTab("p2");
assert.deepEqual(pinnedTabIds.value, ["p2"]);
pinnedTabIds.value = ["p1", "p2"];
ensureSinglePinnedTab();
assert.deepEqual(pinnedTabIds.value, ["p1"]);
pinnedTabIds.value = [];

assert.deepEqual([...collapsePinnedIds(new Set(["p1", "p2"]), "p2")], ["p2"]);
assert.deepEqual([...collapsePinnedIds(new Set(["p1", "p2"]))], ["p1"]);
assert.equal(collapsePinnedIds(new Set(["p1"])).size, 1);

// Shared / empty ids: pinning one must not light every strip entry.
const collided = ensureUniqueTabIds(
  state(
    [
      [tab("dup", "C:\\Temp\\afm-fixture\\Downloads")],
      [tab("dup", "C:\\Temp\\afm-fixture\\Administrator")],
      [tab("dup", "C:\\Temp\\afm-fixture\\awam-skills")],
      [tab("ok", "C:\\Temp\\afm-fixture\\OpenMontage")],
    ],
    0,
  ),
);
const collidedIds = collided.tabGroups.map((group) => group[0]?.id);
assert.equal(new Set(collidedIds).size, 4);
assert.equal(collidedIds[0], "dup");
assert.notEqual(collidedIds[1], "dup");
assert.notEqual(collidedIds[2], "dup");

const emptyIds = normalizeRestoredOpenState(
  state([[tab("", "C:\\a")], [tab("", "C:\\b")], [tab("keep", "C:\\c")]], 0),
);
const emptyNormalized = emptyIds.tabGroups.map((group) => group[0]?.id);
assert.ok(emptyNormalized.every((id) => Boolean(id)));
assert.equal(new Set(emptyNormalized).size, 3);

assert.equal(isTabPinned(""), false);
togglePinnedTab("");
assert.deepEqual(pinnedTabIds.value, []);

const live = state(
  [
    [tab("p1", "C:\\Temp\\afm-fixture\\pin-a")],
    [tab("u1", "C:\\Temp\\afm-fixture\\old")],
    [tab("p2", "C:\\Temp\\afm-fixture\\pin-b")],
    [tab("u2", "C:\\Temp\\afm-fixture\\old-2")],
  ],
  2,
);
const restored = state(
  [
    [tab("t1", "D:\\Temp\\afm-fixture\\one")],
    [tab("t2", "D:\\Temp\\afm-fixture\\two")],
    [tab("t3", "D:\\Temp\\afm-fixture\\three")],
  ],
  1,
);

// Multi-pin input collapses to the active pin (p2) before merge/strip.
const multiPinned = new Set(["p1", "p2"]);
const merged = mergePinnedIntoOpenState(restored, live, multiPinned);
assert.deepEqual(
  merged.tabGroups.map((group) => group[0]?.id),
  ["t1", "t2", "p2", "t3"],
);
assert.equal(merged.currentTabGroupIndex, 2);
assert.equal(
  merged.tabGroups.filter((group) => group[0]?.id === "p1" || group[0]?.id === "p2").length,
  1,
);

const saved = openStateWithoutPinned(live, multiPinned);
assert.deepEqual(
  saved.tabGroups.map((group) => group[0]?.id),
  ["p1", "u1", "u2"],
);

const singlePinned = new Set(["p1"]);
const mergedSingle = mergePinnedIntoOpenState(restored, live, singlePinned);
assert.deepEqual(
  mergedSingle.tabGroups.map((group) => group[0]?.id),
  ["p1", "t1", "t2", "t3"],
);

const untouched = mergePinnedIntoOpenState(restored, live, new Set());
assert.equal(untouched.tabGroups.length, 3);
assert.equal(untouched.currentTabGroupIndex, 1);

const duplicated = mergePinnedIntoOpenState(
  state([[tab("p1", "C:\\Temp\\afm-fixture\\pin-a")], [tab("t1", "D:\\Temp\\afm-fixture\\one")]], 0),
  live,
  singlePinned,
);
assert.equal(duplicated.tabGroups.filter((group) => group[0]?.id === "p1").length, 1);

assert.deepEqual(openStateWithoutPinned(createEmptyOpenState(), singlePinned).tabGroups, []);

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

// FIL-68：固定标签按路径持久化，冷启动可恢复并带入空工作组 strip。
memoryStore.clear();
pinnedTabIds.value = [];
writePinnedSnapshots([]);
assert.deepEqual(readPinnedSnapshots(), []);

const persistLive = state(
  [
    [tab("keep", "C:\\Temp\\afm-fixture\\keep")],
    [tab("pin", "C:\\Temp\\afm-fixture\\pinned")],
  ],
  0,
);
pinnedTabIds.value = ["pin"];
commitPinnedPersistence(persistLive);
assert.deepEqual(readPinnedSnapshots(), [
  {
    path: "C:\\Temp\\afm-fixture\\pinned",
    type: "directory",
    name: "pin",
    stripIndex: 1,
  },
]);

pinnedTabIds.value = [];
const coldRestored = state(
  [[tab("wg", "D:\\Temp\\afm-fixture\\wg-only")]],
  0,
);
const hydrated = hydratePersistedPins(coldRestored);
assert.deepEqual(
  hydrated.state.tabGroups.map((group) => group[0]?.path),
  ["D:\\Temp\\afm-fixture\\wg-only", "C:\\Temp\\afm-fixture\\pinned"],
);
assert.equal(hydrated.state.currentTabGroupIndex, 0);
assert.equal(hydrated.pinnedIds.length, 1);
assert.equal(isTabPinned(hydrated.pinnedIds[0]!), true);

const emptyHydrated = hydratePersistedPins(createEmptyOpenState());
assert.equal(emptyHydrated.state.tabGroups.length, 1);
assert.equal(
  emptyHydrated.state.tabGroups[0]?.[0]?.path,
  "C:\\Temp\\afm-fixture\\pinned",
);
assert.equal(isTabPinned(emptyHydrated.pinnedIds[0]!), true);

togglePinnedTab(emptyHydrated.pinnedIds[0]!);
assert.deepEqual(readPinnedSnapshots(), []);

console.log("tab-pin checks OK.");
