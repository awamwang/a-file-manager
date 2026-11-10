// SPDX-License-Identifier: GPL-3.0-or-later
/** 跨应用拖动（Explorer 互通）Hybrid MVP 守卫：OLE 出站 + HTML5/路径入站 + focus 适配 + FIL-37/38/39。 */
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
assert.ok(
  dragOut.includes("afm://dnd/ole-promote") || dragOut.includes("OLE_PROMOTE"),
  "FIL-39 Hybrid listens for host OLE promote",
);
assert.ok(
  /EDGE_MARGIN_PX\s*=\s*(2[4-9]|[3-9]\d)/.test(dragOut),
  "FIL-39 near-edge margin is at least 24px",
);
assert.ok(!/setFocus\s*\(/.test(dragOut), "FIL-37 outbound path must not call setFocus");
assert.ok(paths.includes("pathsFromDataTransfer"), "inbound path decode");
assert.ok(paths.includes("fileUrlToPath"), "Explorer file:// URLs");
assert.ok(paths.includes("text/uri-list"), "uri-list fallback");
assert.ok(surface.includes("dataTransferLooksExternal"), "external vs in-app");
assert.ok(surface.includes("commitFolderDrop"), "shared inbound commit");
assert.ok(focus.includes("MA_NOACTIVATE") || focus.includes("note_drag_gesture"), "drag_focus adapter");
assert.ok(focus.includes("afm://dnd/ole-promote"), "FIL-39 host emits OLE promote");
assert.ok(focus.includes("WindowFromPoint"), "FIL-39 cursor not over root HWND");
assert.ok(focus.includes("LockSetForegroundWindow"), "FIL-37 documents LockSetForegroundWindow tradeoff");
assert.ok(existsSync(patchWin), "patched drag crate Windows OLE path");
const patch = readFileSync(patchWin, "utf8");
assert.ok(patch.includes("DoDragDrop"), "patched DoDragDrop");
assert.ok(patch.includes("CF_HDROP"), "patched CF_HDROP");
assert.ok(patch.includes("BHID_DataObject"), "FIL-38 Files path uses Shell BHID_DataObject");
assert.ok(patch.includes("CFSTR_SHELLIDLIST"), "FIL-38 audits Shell IDList format");
assert.ok(
  patch.includes("shell_file_formats_gettable") || patch.includes("shell_data_object_exposes"),
  "FIL-38 format audit helper/test present",
);
assert.ok(
  patch.includes("手写 IDataObject") || patch.includes("FIL-38"),
  "FIL-38 documents isolation/removal of handwritten DataObject",
);
assert.ok(
  !/struct DataObject\s*\{/.test(patch),
  "FIL-38 handwritten DataObject must not remain on Files path",
);
assert.ok(patch.includes("SetWindowPos"), "FIL-37 OLE DropSource restores z-order");
assert.ok(design.includes("Hybrid outbound"), "design doc names Hybrid outbound");
assert.ok(design.includes("shell inbound") || design.includes("inbound"), "design doc names inbound");
assert.ok(
  design.includes("FIL-37") || design.includes("LockSetForegroundWindow"),
  "design doc records FIL-37 focus / upgrade gate",
);

console.log(
  "cross-app DnD Hybrid OK (Explorer OLE out + shell inbound + FIL-37/38/39 guards).",
);
