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
assert.ok(view.includes("isIconView.value ? 2 : 1"));
assert.ok(view.includes("field-sizing: content"));
assert.ok(view.includes("min-height: 0"));
assert.ok(view.includes("@mousedown.stop"));
assert.ok(view.includes("cursor: text"));
assert.ok(view.includes("input.style.width"));
// Explorer-like focus: mouse leave / transient blur keep rename; outside pointerdown commits.
assert.ok(view.includes("onRenameOutsidePointerDown"));
assert.ok(view.includes("armRenameOutsideListener"));
assert.ok(view.includes("disarmRenameOutsideListener"));
assert.ok(view.includes('addEventListener("pointerdown", onRenameOutsidePointerDown, true)'));
assert.ok(view.includes("input.focus({ preventScroll: true })"));
{
  const blurStart = view.indexOf("function onRenameBlur");
  assert.ok(blurStart >= 0, "onRenameBlur required");
  const blurEnd = view.indexOf("\nasync function commitInlineRename", blurStart);
  const blurBody = view.slice(blurStart, blurEnd > blurStart ? blurEnd : blurStart + 400);
  assert.ok(!blurBody.includes("cancelInlineRename"), "blur must not cancel rename");
  assert.ok(blurBody.includes("input.focus"), "blur must restore focus while renaming");
}
assert.ok(view.includes("if (renamingPath.value) return;"));
// Drag-select must scroll through overflow — never clip with overflow:hidden.
assert.ok(view.includes("overflow-x: auto"));
assert.ok(view.includes("overflow-y: hidden"));
assert.ok(view.includes("overflow-y: auto"));
assert.ok(view.includes("user-select: text"));
assert.ok(view.includes("scrollbar-width: none"));
assert.match(view, /\.inline-rename\s*\{[^}]*overflow-x:\s*auto/s);
assert.ok(!/\.inline-rename\s*\{[^}]*overflow:\s*hidden/s.test(view));
assert.ok(!/\.inline-rename\s*\{[^}]*min-width:\s*10rem/s.test(view));
assert.ok(!/\.inline-rename\s*\{[^}]*min-height:\s*var\(--fluent-control-height/s.test(view));
assert.ok(!view.includes("window.prompt(t(\"contextMenu.renamePrompt\")"));
assert.ok(!view.includes("window.prompt(t('contextMenu.renamePrompt')"));

// Context Rename delegates to parent inline edit.
assert.ok(menu.includes("requestRename"));
assert.ok(menu.includes('emit("requestRename", entry.path)'));
assert.ok(!menu.includes("runRename"));
assert.ok(!menu.includes("window.prompt(t(\"contextMenu.renamePrompt\")"));
assert.ok(!menu.includes("renameEntry"));

console.log("Navigator inline rename checks OK (AIA-171/AIA-172/AIA-173).");
