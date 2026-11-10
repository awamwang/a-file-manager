// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-270: guard removed unused exports / modules. */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

assert.ok(
  !existsSync(join(root, "src/state/expandEnvPath.ts")),
  "expandEnvPath.ts removed — address expand is Rust-only",
);

const browse = read("src/state/navigatorBrowse.ts");
assert.ok(!browse.includes("navigatorTabs"), "deprecated navigatorTabs removed");
assert.ok(browse.includes("stripTabs"), "stripTabs remains");

const panel = read("src/state/commandPanel.ts");
assert.ok(!panel.includes("openCommandPanel"), "unused openCommandPanel removed");
assert.ok(panel.includes("toggleCommandPanel") && panel.includes("closeCommandPanel"));

const undo = read("src/state/fileUndo/index.ts");
assert.ok(!undo.includes("canUndo"), "unused canUndo removed");
assert.ok(!undo.includes("clearUndoStack"), "unused clearUndoStack removed");
assert.ok(!undo.includes("peekUndoStack"), "unused peekUndoStack removed");
assert.ok(undo.includes("undoLast") && undo.includes("recordCopyUndo"));

const dragOut = read("src/state/fileDragOut.ts");
assert.ok(
  !dragOut.includes("export function disarmNativeFileDragOut"),
  "disarmNativeFileDragOut stays private",
);
assert.ok(dragOut.includes("function disarmNativeFileDragOut"));

console.log("Dead-code cleanup checks OK (AIA-270).");
