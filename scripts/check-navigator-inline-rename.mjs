// SPDX-License-Identifier: GPL-3.0-or-later
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

// Inline rename UI — no prompt modal for rename.
assert.ok(view.includes("startInlineRename"));
assert.ok(view.includes("commitInlineRename"));
assert.ok(view.includes("cancelInlineRename"));
assert.ok(view.includes("onRenameBlur"));
assert.ok(view.includes("onNameClick"));
assert.ok(view.includes("syncRenameInputSize"));
assert.ok(view.includes("onRenameDraftInput"));
assert.ok(view.includes("HTMLTextAreaElement"));
assert.ok(view.includes('class="fluent-control inline-rename"'));
assert.ok(view.includes("<textarea"));
assert.ok(view.includes('rows="1"'));
assert.ok(view.includes('@keydown.enter.prevent.stop="commitInlineRename"'));
assert.ok(view.includes('@keydown.escape.prevent.stop="cancelInlineRename"'));
assert.ok(view.includes('registerShortcutHandler("navigator.rename"'));
assert.ok(view.includes("min-width: 10rem"));
assert.ok(view.includes("isIconView.value ? 2 : 1"));
assert.ok(!view.includes("window.prompt(t(\"contextMenu.renamePrompt\")"));
assert.ok(!view.includes("window.prompt(t('contextMenu.renamePrompt')"));

// Context Rename delegates to parent inline edit.
assert.ok(menu.includes("requestRename"));
assert.ok(menu.includes('emit("requestRename", entry.path)'));
assert.ok(!menu.includes("runRename"));
assert.ok(!menu.includes("window.prompt(t(\"contextMenu.renamePrompt\")"));
assert.ok(!menu.includes("renameEntry"));

console.log("Navigator inline rename checks OK (AIA-171/AIA-172).");
