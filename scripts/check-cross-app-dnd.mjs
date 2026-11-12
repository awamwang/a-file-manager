// SPDX-License-Identifier: GPL-3.0-or-later
/** 跨应用拖动（Explorer 互通）守卫：OLE 出站 + HTML5/路径入站 + focus + FIL-37/38/39/42/43/44/45。 */
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
const nav = read("src/views/NavigatorView.vue");
const patchWin = join(root, "src-tauri/patches/drag/src/platform_impl/windows/mod.rs");

assert.ok(dragOut.includes("startDrag"), "outbound arms tauri-plugin-drag startDrag");
assert.ok(dragOut.includes("CF_HDROP") || dragOut.includes("OLE"), "outbound documents OLE/CF_HDROP");
assert.ok(
  dragOut.includes("armOutboundPathsForPress") && dragOut.includes("dnd_arm_outbound_paths"),
  "FIL-45 press arms host outbound paths",
);
assert.ok(!/setFocus\s*\(/.test(dragOut), "FIL-37 outbound path must not call setFocus");
assert.ok(paths.includes("pathsFromDataTransfer"), "inbound path decode");
assert.ok(paths.includes("fileUrlToPath"), "Explorer file:// URLs");
assert.ok(paths.includes("text/uri-list"), "uri-list inbound fallback");
assert.ok(
  paths.includes("setOutboundPathPayload") &&
    !/setData\(\s*["']text\/plain["']/.test(paths) &&
    !/setData\(\s*["']text\/uri-list["']/.test(paths),
  "FIL-45 outbound HTML5 must not set text/plain or text/uri-list",
);
assert.ok(surface.includes("dataTransferLooksExternal"), "external vs in-app");
assert.ok(surface.includes("commitFolderDrop"), "shared inbound commit");
assert.ok(focus.includes("MA_NOACTIVATE") || focus.includes("note_drag_gesture"), "drag_focus adapter");
assert.ok(focus.includes("WS_EX_NOACTIVATE") && focus.includes("enable_temp_noactivate"), "FIL-45 temp WS_EX_NOACTIVATE");
assert.ok(
  focus.includes("start_files_drag_hwnd") && focus.includes("take_outbound_paths"),
  "FIL-45 host promote starts OLE in-process when paths are armed",
);
assert.ok(
  focus.includes("Chrome_WidgetWin_") && focus.includes("webview_subclass_proc"),
  "FIL-43 subclasses WebView2 child HWNDs for press-phase MA_NOACTIVATE",
);
assert.ok(
  focus.includes("enable_temp_noactivate") &&
    focus.includes("disable_temp_noactivate") &&
    focus.includes("临时"),
  "FIL-45 WS_EX_NOACTIVATE must be temporary (enable/disable around press)",
);
assert.ok(existsSync(patchWin), "patched drag crate Windows OLE path");
const patch = readFileSync(patchWin, "utf8");
assert.ok(patch.includes("DoDragDrop"), "patched DoDragDrop");
assert.ok(patch.includes("CF_HDROP"), "patched CF_HDROP");
assert.ok(patch.includes("BHID_DataObject"), "FIL-38 Files path uses Shell BHID_DataObject");
assert.ok(patch.includes("CFSTR_SHELLIDLIST"), "FIL-38 audits Shell IDList format");
assert.ok(
  patch.includes("CFSTR_PREFERREDDROPEFFECT") || patch.includes("Preferred DropEffect"),
  "FIL-42 sets Preferred DropEffect on Shell IDataObject",
);
assert.ok(
  patch.includes("IDataObjectAsyncCapability") && patch.includes("SetAsyncMode"),
  "FIL-42 disables async capability so Drop GetData stays synchronous",
);
assert.ok(
  !/if\s*\(.*PotPlayer/i.test(patch) && !/target\s*==\s*["']PotPlayer/i.test(patch),
  "FIL-42 must not add PotPlayer-specific format hacks",
);
assert.ok(patch.includes("AttachThreadInput") && patch.includes("LockSetForegroundWindow"), "FIL-44 FG restore");
assert.ok(patch.includes("start_files_drag_hwnd") && patch.includes("arm_outbound_paths"), "FIL-44 host OLE API");

assert.ok(nav.includes("preventDefault") && nav.includes("onDragStart"), "Navigator dragstart present");
const dragStart = nav.match(/function onDragStart\([^)]*\)\s*\{[\s\S]*?\n\}/);
assert.ok(dragStart, "onDragStart function");
assert.ok(
  dragStart[0].includes("preventDefault"),
  "FIL-45: onDragStart must preventDefault to cancel HTML5 outbound",
);
assert.ok(
  nav.includes("armOutboundPathsForPress") && nav.includes("startNativeFileDragOut"),
  "FIL-45 Navigator arms press paths and starts OLE",
);

assert.ok(design.includes("FIL-45"), "design doc records FIL-45");
assert.ok(design.includes("FIL-46") || design.includes("WH_MOUSE_LL"), "design doc records FIL-46");
assert.ok(
  focus.includes("WH_MOUSE_LL") && focus.includes("mouse_ll_proc"),
  "FIL-46 installs WH_MOUSE_LL to arm before WebView raise",
);
assert.ok(
  focus.includes("GetAncestor") && focus.includes("GA_ROOT"),
  "FIL-46 hwnd_is_ours uses GetAncestor for OOP WebView",
);
assert.ok(
  focus.includes("WA_INACTIVE") && focus.includes("remember_other_foreground"),
  "FIL-46 enables noactivate on deactivate and remembers other FG",
);
assert.ok(
  !/SetWindowPos\(\s*\w+\s*,\s*HWND_TOPMOST/.test(patch) &&
    !patch.includes("HWND_NOTOPMOST"),
  "FIL-47 must not SetWindowPos TOPMOST on the drop target",
);
assert.ok(
  patch.includes("我们不是 FG") || patch.includes("!hwnd_is_ours(our_root, fg)"),
  "FIL-47 restore must no-op unless AFM stole foreground",
);
assert.ok(design.includes("FIL-47"), "design doc records FIL-47 retract TOPMOST");

console.log(
  "cross-app DnD Hybrid OK (Explorer OLE out + shell inbound + FIL-37..47 guards).",
);
