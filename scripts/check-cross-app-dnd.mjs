// SPDX-License-Identifier: GPL-3.0-or-later
/** 跨应用拖动（Explorer 互通）Hybrid MVP 守卫：OLE 出站 + HTML5/路径入站 + focus 适配。 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const dragOut = read("src/state/dnd/dragOut.ts");
const paths = read("src/state/dnd/paths.ts");
const surface = read("src/state/dnd/surface.ts");
const focus = read("src-tauri/src/drag_focus.rs");
const design = read("docs/afm-explorer-dnd-interop-design.md");
const patchWin = join(root, "src-tauri/patches/drag/src/platform_impl/windows/mod.rs");

assert.ok(dragOut.includes("startDrag"), "outbound arms tauri-plugin-drag startDrag");
assert.ok(dragOut.includes("CF_HDROP") || dragOut.includes("OLE"), "outbound documents OLE/CF_HDROP");
assert.ok(paths.includes("pathsFromDataTransfer"), "inbound path decode");
assert.ok(paths.includes("fileUrlToPath"), "Explorer file:// URLs");
assert.ok(paths.includes("text/uri-list"), "uri-list fallback");
assert.ok(surface.includes("dataTransferLooksExternal"), "external vs in-app");
assert.ok(surface.includes("commitFolderDrop"), "shared inbound commit");
assert.ok(focus.includes("MA_NOACTIVATE") || focus.includes("note_drag_gesture"), "drag_focus adapter");
assert.ok(existsSync(patchWin), "patched drag crate Windows OLE path");
const patch = readFileSync(patchWin, "utf8");
assert.ok(patch.includes("DoDragDrop"), "patched DoDragDrop");
assert.ok(patch.includes("CF_HDROP"), "patched CF_HDROP");
assert.ok(design.includes("Hybrid outbound"), "design doc names Hybrid outbound");
assert.ok(design.includes("shell inbound") || design.includes("inbound"), "design doc names inbound");

console.log("cross-app DnD Hybrid MVP OK (Explorer OLE out + shell inbound).");
