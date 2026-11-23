// SPDX-License-Identifier: GPL-3.0-or-later
// Shift range uses the current sorted/filtered view order, not raw disk order (SEL-004).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const selection = readFileSync(join(root, "src/state/navigatorSelection.ts"), "utf8");
const pkg = readFileSync(join(root, "package.json"), "utf8");

// orderedPaths is derived from visibleEntries (already sorted + filtered), so the view order
// is what feeds range selection.
assert.match(
  view,
  /const orderedPaths = computed\(\(\) => visibleEntries\.value\.map\(\(e\) => e\.path\)\);/,
);
// Shift+click extends from the anchor to the target across that ordered view list.
assert.ok(
  view.includes("selectRange(orderedPaths.value, selectionAnchor.value, entry.path)"),
);
// selectRange slices the ordered array between anchor and target indices.
assert.match(
  selection,
  /export function selectRange\([\s\S]*?orderedPaths\.slice\(start, end \+ 1\);/,
);

assert.ok(pkg.includes('"test:selection-view-order"'));

console.log("selection-view-order OK (SEL-004).");
