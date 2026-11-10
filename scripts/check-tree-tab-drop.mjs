// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-242: folder-tree + tab strip file drop surfaces + hover expand/activate.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const hover = read("src/state/dnd/hover.ts");
const surface = read("src/state/dnd/surface.ts");
const branch = read("src/components/shell/FolderTreeBranch.vue");
const shell = read("src/components/shell/AppShell.vue");
const nav = read("src/views/NavigatorView.vue");

assert.ok(hover.includes("DROP_HOVER_MS = 500"), "Explorer-like 500ms dwell");
assert.ok(hover.includes("createDropHoverScheduler"), "hover scheduler export");

assert.ok(surface.includes("acceptFolderDragOver"), "shared dragover");
assert.ok(surface.includes("commitFolderDrop"), "shared drop commit");
assert.ok(surface.includes("previewDropMode"), "shared mode preview");
assert.ok(surface.includes("refreshActiveListing"), "refresh after drop");
assert.ok(surface.includes("transferEntries"), "uses file ops transfer");
assert.ok(surface.includes("navigator.copiedInto"), "copy notice key");
assert.ok(surface.includes("navigator.movedInto"), "move notice key");
assert.ok(surface.includes("navigator.linkedInto"), "link notice key");

assert.ok(branch.includes("onTreeDragOver"), "tree dragover");
assert.ok(branch.includes("onTreeDrop"), "tree drop");
assert.ok(branch.includes("hoverExpand"), "tree hover expand");
assert.ok(branch.includes('data-drop-target="folder-tree"'), "tree drop marker");
assert.ok(branch.includes("expandAndLoad"), "expand on hover");
assert.ok(branch.includes("createDropHoverScheduler"), "tree uses hover scheduler");

assert.ok(shell.includes("onTabFileDragOver"), "tab file dragover");
assert.ok(shell.includes("onTabFileDrop"), "tab file drop");
assert.ok(shell.includes("tabHoverActivate"), "tab hover activate");
assert.ok(shell.includes("switchNavigatorTab"), "tab activate switches");
assert.ok(shell.includes('data-drop-target="tab"'), "tab drop marker");
assert.ok(shell.includes("tab--file-drop"), "tab drop visual");
assert.ok(shell.includes("tabAcceptsFileDrop"), "reject guide/recycle tabs");
assert.ok(shell.includes("isGuidePath") && shell.includes("isRecyclePath"), "guide/recycle helpers");

assert.ok(nav.includes("fileDropSurface") || nav.includes("state/dnd"), "list reuses shared dnd surface");
assert.ok(nav.includes("commitFolderDrop"), "list drop via shared commit");
assert.ok(nav.includes("acceptFolderDragOver"), "list dragover via shared accept");

const runtime = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/tree-tab-drop-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (runtime.stdout) process.stdout.write(runtime.stdout);
if (runtime.stderr) process.stderr.write(runtime.stderr);
if (runtime.status !== 0) process.exit(runtime.status ?? 1);

console.log("Tree/tab drop surface checks OK (AIA-242).");
