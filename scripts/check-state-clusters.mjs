// SPDX-License-Identifier: GPL-3.0-or-later
/** 概念聚类：相关纯模块与编排模块落在同一 state 子目录。 */
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const state = join(root, "src", "state");

const required = [
  "recycle/path.ts",
  "recycle/listing.ts",
  "recycle/bin.ts",
  "recycle/index.ts",
  "showHidden/flag.ts",
  "showHidden/index.ts",
  "dualPane/layout.ts",
  "dualPane/index.ts",
  "fileUndo/stack.ts",
  "fileUndo/index.ts",
  "guide/path.ts",
  "guide/index.ts",
  "workGroup/name.ts",
  "workGroup/chrome.ts",
  "workGroup/index.ts",
  "dnd/mode.ts",
  "dnd/paths.ts",
  "dnd/hover.ts",
  "dnd/surface.ts",
  "dnd/dragOut.ts",
  "dnd/index.ts",
  "openState/model.ts",
  "openState/session.ts",
  "openState/index.ts",
];

for (const rel of required) {
  assert.ok(existsSync(join(state, rel)), `missing cluster file: ${rel}`);
}

const forbiddenFlat = [
  "recyclePath.ts",
  "recycleListing.ts",
  "recycleBin.ts",
  "showHiddenFlag.ts",
  "dualPaneLayout.ts",
  "fileUndoStack.ts",
  "guidePath.ts",
  "navigatorGuide.ts",
  "workGroupName.ts",
  "workGroupChrome.ts",
  "dropMode.ts",
  "dropPaths.ts",
  "dropHover.ts",
  "fileDropSurface.ts",
  "fileDragOut.ts",
  "openStateModel.ts",
];

for (const rel of forbiddenFlat) {
  assert.ok(!existsSync(join(state, rel)), `flat leftover should be gone: ${rel}`);
}

console.log("state concept clusters OK.");
