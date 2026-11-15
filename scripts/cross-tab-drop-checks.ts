// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { dropTransferMode, volumeKey } from "../src/state/dnd/mode.ts";
import {
  createDropHoverScheduler,
  DROP_HOVER_MS,
  TAB_DROP_HOVER_MS,
} from "../src/state/dnd/hover.ts";
import {
  activeDragSources,
  beginDragSources,
  dropHint,
  endDragSession,
  hideDropHint,
  showDropHint,
} from "../src/state/dnd/session.ts";

assert.equal(DROP_HOVER_MS, 500, "tree/places dwell is 500ms");
assert.equal(TAB_DROP_HOVER_MS, 500, "tab dwell matches tree/places at 500ms");

// Explorer rule: same volume = cut/move, another volume = copy; modifiers win.
assert.equal(dropTransferMode(["C:\\Src\\a.txt"], "C:\\Dst"), "move");
assert.equal(dropTransferMode(["C:\\Src\\a.txt"], "D:\\Dst"), "copy");
assert.equal(
  dropTransferMode(["C:\\Src\\a.txt", "D:\\other.txt"], "C:\\Dst"),
  "copy",
  "mixed sources are a cross-volume copy",
);
assert.equal(dropTransferMode(["C:\\Src\\a.txt"], "C:\\Dst", { ctrl: true }), "copy");
assert.equal(dropTransferMode(["C:\\Src\\a.txt"], "D:\\Dst", { shift: true }), "move");
assert.equal(dropTransferMode(["C:\\Src\\a.txt"], "D:\\Dst", { alt: true }), "link");
assert.equal(volumeKey("d:/Work"), "d");
assert.equal(volumeKey("\\\\nas\\share"), "unc:nas");

// The 500ms scheduler must not fire early, and clear() must cancel a pending switch.
await new Promise<void>((resolve, reject) => {
  const sched = createDropHoverScheduler(TAB_DROP_HOVER_MS);
  let fired = 0;
  sched.schedule("tab:1", () => {
    fired += 1;
  });
  sched.schedule("tab:1", () => {
    fired += 10;
  });
  setTimeout(() => {
    try {
      assert.equal(fired, 0, "no tab switch before 500ms");
      sched.clear();
      setTimeout(() => {
        try {
          assert.equal(fired, 0, "clear cancels the pending tab switch");
          resolve();
        } catch (err) {
          reject(err);
        }
      }, 30);
    } catch (err) {
      reject(err);
    }
  }, 120);
});

// A full 500ms dwell does activate the hovered tab.
await new Promise<void>((resolve, reject) => {
  const sched = createDropHoverScheduler(TAB_DROP_HOVER_MS);
  const watchdog = setTimeout(() => reject(new Error("500ms dwell did not fire")), 900);
  sched.schedule("tab:positive", () => {
    clearTimeout(watchdog);
    resolve();
  });
});

// Session sources survive a tab switch (selection is cleared there).
endDragSession();
assert.deepEqual(activeDragSources(), []);
beginDragSources(["C:\\Src\\a.txt", "C:\\Src\\b.txt", "C:\\SRC\\A.TXT"]);
assert.deepEqual(activeDragSources(), ["C:\\Src\\a.txt", "C:\\Src\\b.txt"]);
assert.deepEqual(dropTransferMode(activeDragSources(), "C:\\Dst"), "move");
assert.deepEqual(dropTransferMode(activeDragSources(), "D:\\Dst"), "copy");

// Hint follows the cursor and only its own target can clear it.
showDropHint(
  { clientX: 100, clientY: 200, currentTarget: null } as unknown as DragEvent,
  "move",
  "C:\\Work",
  "tab:2",
);
assert.equal(dropHint.value?.mode, "move");
assert.equal(dropHint.value?.x, 116);
assert.equal(dropHint.value?.y, 218);
assert.equal(dropHint.value?.target, "C:\\Work");
hideDropHint("tab:1");
assert.ok(dropHint.value, "another target's dragleave must not clear the hint");
hideDropHint("tab:2");
assert.equal(dropHint.value, null);

// Native OLE drags may omit coordinates: fall back to the target rect centre.
showDropHint(
  {
    clientX: 0,
    clientY: 0,
    currentTarget: {
      getBoundingClientRect: () => ({ left: 10, top: 20, width: 100, height: 40 }),
    },
  } as unknown as DragEvent,
  "copy",
  "D:\\Work",
  "list",
);
assert.equal(dropHint.value?.x, 76);
assert.equal(dropHint.value?.y, 58);
assert.equal(dropHint.value?.mode, "copy");

// 光标轻微抖动不重建浮层，模式切换则立即更新文案。
const steady = dropHint.value;
showDropHint(
  { clientX: 61, clientY: 41, currentTarget: null } as unknown as DragEvent,
  "copy",
  "D:\\Work",
  "list",
);
assert.equal(dropHint.value, steady, "cursor jitter must not rebuild the hint");
showDropHint(
  { clientX: 61, clientY: 41, currentTarget: null } as unknown as DragEvent,
  "move",
  "D:\\Work",
  "list",
);
assert.equal(dropHint.value?.mode, "move", "changing mode updates the hint in place");

endDragSession();
assert.deepEqual(activeDragSources(), []);
assert.equal(dropHint.value, null);

console.log("cross-tab-drop runtime checks OK.");
