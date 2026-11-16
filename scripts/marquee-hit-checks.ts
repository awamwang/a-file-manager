// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  hitTestMarqueeEntries,
  marqueeHitMode,
  rectsIntersect,
  resolveMarqueeHitRect,
} from "../src/state/marqueeHitTest.ts";

assert.equal(rectsIntersect({ left: 0, top: 0, right: 10, bottom: 10 }, { left: 9, top: 9, right: 20, bottom: 20 }), true);
assert.equal(rectsIntersect({ left: 0, top: 0, right: 10, bottom: 10 }, { left: 10, top: 0, right: 20, bottom: 10 }), false);
assert.equal(rectsIntersect({ left: 0, top: 0, right: 10, bottom: 10 }, { left: 11, top: 0, right: 20, bottom: 10 }), false);

assert.equal(marqueeHitMode({ isIconView: true, isTilesView: false }), "icon");
assert.equal(marqueeHitMode({ isIconView: false, isTilesView: true }), "icon");
assert.equal(marqueeHitMode({ isIconView: false, isTilesView: false }), "row");

// 框覆盖四个图标槽全覆盖、第五个在框外 → 四个命中（相交即选，非完全包含）。
const icons = [
  { path: "a", hitRect: { left: 10, top: 10, right: 42, bottom: 42 } },
  { path: "b", hitRect: { left: 60, top: 10, right: 92, bottom: 42 } },
  { path: "c", hitRect: { left: 110, top: 10, right: 142, bottom: 42 } },
  { path: "d", hitRect: { left: 160, top: 10, right: 192, bottom: 42 } },
  { path: "e", hitRect: { left: 210, top: 10, right: 242, bottom: 42 } },
];
const marquee = { left: 8, top: 8, right: 200, bottom: 50 };
assert.deepEqual(hitTestMarqueeEntries(marquee, icons), ["a", "b", "c", "d"]);

// 仅擦到 e 图标左缘也应选中（相交，非包含）。
assert.deepEqual(
  hitTestMarqueeEntries({ left: 240, top: 20, right: 300, bottom: 40 }, icons),
  ["e"],
);

// 选框太小（空选阈值）忽略。
assert.deepEqual(hitTestMarqueeEntries({ left: 0, top: 0, right: 1, bottom: 1 }, icons), []);

// 行模式：整行矩形相交。
const rows = [
  { path: "r1", hitRect: { left: 0, top: 0, right: 400, bottom: 28 } },
  { path: "r2", hitRect: { left: 0, top: 28, right: 400, bottom: 56 } },
  { path: "r3", hitRect: { left: 0, top: 56, right: 400, bottom: 84 } },
];
assert.deepEqual(
  hitTestMarqueeEntries({ left: 10, top: 20, right: 80, bottom: 70 }, rows),
  ["r1", "r2", "r3"],
);

// 图标模式：相交图标槽即选中。
const rowRect = { left: 0, top: 0, right: 100, bottom: 120 };
const kindRect = { left: 34, top: 8, right: 66, bottom: 40 };
assert.deepEqual(
  resolveMarqueeHitRect("icon", rowRect, kindRect),
  kindRect,
);
assert.deepEqual(
  hitTestMarqueeEntries({ left: 40, top: 10, right: 50, bottom: 20 }, [
    { path: "icon-hit", hitRect: resolveMarqueeHitRect("icon", rowRect, kindRect)! },
  ]),
  ["icon-hit"],
);
// 选框只盖住文件名区、未碰到图标 → 不选中。
assert.deepEqual(
  hitTestMarqueeEntries({ left: 10, top: 80, right: 90, bottom: 110 }, [
    { path: "name-only", hitRect: resolveMarqueeHitRect("icon", rowRect, kindRect)! },
  ]),
  [],
);

// content-visibility 等导致图标槽 0×0 时回退整行，避免框选全滅。
const zeroKind = { left: 0, top: 0, right: 0, bottom: 0 };
assert.deepEqual(
  resolveMarqueeHitRect("icon", rowRect, zeroKind),
  rowRect,
);
assert.deepEqual(
  hitTestMarqueeEntries({ left: 10, top: 80, right: 90, bottom: 110 }, [
    {
      path: "fallback-row",
      hitRect: resolveMarqueeHitRect("icon", rowRect, zeroKind)!,
    },
  ]),
  ["fallback-row"],
);

console.log("Marquee hit-test checks OK (FIL-73).");
