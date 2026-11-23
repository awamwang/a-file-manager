// SPDX-License-Identifier: GPL-3.0-or-later
// Post-state after copy/move/paste (LC-001/002, CLIP-003/004) + LC-011 scroll/sort:
// transfer success selects the freshly written items; a same-directory refresh keeps the
// current sort and does not jump the list to top (scroll reset is keyed to navigation only).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const sort = readFileSync(join(root, "src/state/navigatorSort.ts"), "utf8");
const pkg = readFileSync(join(root, "package.json"), "utf8");

// Keyboard paste selects the new items (reveal-and-select the transfer result).
assert.match(
  view,
  /const result = await pasteClipboard\(pasteTarget\);[\s\S]*?await revealItemsInNavigator\(result\.paths\);/,
);

// Context-menu paste selects the new items too.
assert.match(
  menu,
  /const result = await pasteClipboard\(dest\);[\s\S]*?await revealItemsInNavigator\(result\.paths\);/,
);

// LC-011 sort survives a refresh: sort column/direction are module-level refs, not reset
// when the listing reloads.
assert.ok(sort.includes("export const sortColumn = ref"));
assert.ok(sort.includes("export const sortDirection = ref"));

// LC-011 scroll survives a refresh: the list scrollTop is reset to 0 only on navigation
// (watch(currentPath)), never unconditionally by the file-mutation refresh path.
assert.match(
  view,
  /watch\(currentPath, \(\) => \{[\s\S]*?primaryScrollTop\.value = 0;[\s\S]*?listRoot\.value\.scrollTop = 0;[\s\S]*?\}\);/,
);
const refreshBody = view.slice(
  view.indexOf("async function refreshListing("),
  view.indexOf("async function refreshListing(") + 600,
);
assert.ok(
  !/\.scrollTop = 0/.test(refreshBody),
  "refreshListing must not reset list scroll",
);

assert.ok(pkg.includes('"test:paste-post-state"'));

console.log("paste-post-state OK (LC-001/002/011, CLIP-003/004).");
