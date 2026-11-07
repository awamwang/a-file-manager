// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-240: native OLE/file drag-out with COPY|MOVE|LINK (patched drag crate).
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const caps = readFileSync(join(root, "src-tauri/capabilities/default.json"), "utf8");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const dragOut = readFileSync(join(root, "src/state/fileDragOut.ts"), "utf8");
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const patchWin = join(root, "src-tauri/patches/drag/src/platform_impl/windows/mod.rs");

assert.ok(
  cargo.includes("tauri-plugin-drag"),
  "Cargo.toml must depend on tauri-plugin-drag",
);
assert.ok(
  cargo.includes('drag = { path = "patches/drag" }') || cargo.includes("patches/drag"),
  "Cargo.toml must patch drag crate for LINK mask",
);
assert.ok(existsSync(patchWin), "patched drag windows mod must exist");
const patchSrc = readFileSync(patchWin, "utf8");
assert.ok(patchSrc.includes("DROPEFFECT_LINK"), "patch must include DROPEFFECT_LINK");
assert.ok(
  patchSrc.includes("DROPEFFECT_COPY") && patchSrc.includes("DROPEFFECT_MOVE"),
  "patch must offer COPY|MOVE|LINK",
);
// AIA-261/264/266: outbound OLE must not steal / keep focus over the drop target.
assert.ok(
  patchSrc.includes("GetForegroundWindow") && patchSrc.includes("SetForegroundWindow"),
  "patch must read/restore foreground around DoDragDrop",
);
assert.ok(
  patchSrc.includes("preserve_fg") || patchSrc.includes("restore_preserved_foreground"),
  "patch must preserve prior foreground during OLE",
);
assert.ok(
  patchSrc.includes("OLE_DRAG_IN_PROGRESS") || patchSrc.includes("ole_drag_in_progress"),
  "patch must expose OLE-in-progress for host MA_NOACTIVATE coordination",
);
assert.ok(
  patchSrc.includes("resolve_preserve_foreground") && patchSrc.includes("PRESS_PRIOR_FG"),
  "patch must prefer press-time prior FG when mousedown already raised AFM (AIA-264)",
);
assert.ok(
  patchSrc.includes("note_drag_gesture") && patchSrc.includes("DRAG_GESTURE"),
  "patch must distinguish drag gesture vs click activate (AIA-264)",
);

const dragFocus = join(root, "src-tauri/src/drag_focus.rs");
assert.ok(existsSync(dragFocus), "AIA-261/264/266 drag_focus.rs host subclass must exist");
const dragFocusSrc = readFileSync(dragFocus, "utf8");
assert.ok(dragFocusSrc.includes("WM_MOUSEACTIVATE"), "drag_focus must handle WM_MOUSEACTIVATE");
assert.ok(dragFocusSrc.includes("MA_NOACTIVATE"), "drag_focus must return MA_NOACTIVATE");
assert.ok(dragFocusSrc.includes("SM_CXDRAG") && dragFocusSrc.includes("SM_CYDRAG"), "drag_focus must use system drag threshold");
assert.ok(dragFocusSrc.includes("WM_ACTIVATE"), "drag_focus must undo WebView raise while button down");
assert.ok(dragFocusSrc.includes("note_drag_gesture"), "drag_focus must mark drag past threshold");
assert.ok(
  dragFocusSrc.includes("WM_LBUTTONDOWN"),
  "drag_focus must arm deferred activate on PARENTNOTIFY LBUTTONDOWN (AIA-266)",
);
assert.ok(
  dragFocusSrc.includes("finish_press_maybe_activate"),
  "drag_focus must share mouse-up activate path for PARENTNOTIFY + timer fallback (AIA-266)",
);
assert.ok(
  lib.includes("drag_focus::install_for_main_window"),
  "lib.rs must install drag_focus subclass on main window",
);

assert.ok(lib.includes("tauri_plugin_drag::init()"), "lib.rs must register tauri_plugin_drag");
assert.ok(caps.includes('"drag:default"'), "capabilities must grant drag:default");
assert.ok(
  pkg.dependencies?.["@crabnebula/tauri-plugin-drag"],
  "package.json must depend on @crabnebula/tauri-plugin-drag",
);

assert.ok(dragOut.includes("startDrag"), "fileDragOut must call startDrag");
assert.ok(dragOut.includes("@crabnebula/tauri-plugin-drag"), "import CrabNebula drag plugin");
assert.ok(dragOut.includes("armNativeFileDragOut"), "export armNativeFileDragOut");
assert.ok(
  dragOut.includes("EDGE_MARGIN") || dragOut.includes("isOutsideOrNearEdge"),
  "must promote OLE near window edge",
);
assert.ok(dragOut.includes("dragleave"), "must promote OLE on document dragleave");
assert.ok(dragOut.includes("resolveResource"), "must resolve drag preview icon");

assert.ok(nav.includes("armNativeFileDragOut"), "NavigatorView must arm native drag-out");
assert.ok(nav.includes("setOutboundPathPayload"), "NavigatorView must keep HTML5 payload");
assert.ok(nav.includes('effectAllowed = "all"'), "dragstart should allow copy+move+link");
assert.ok(
  nav.includes("onRowMouseDown") && nav.includes("afmWindowFocused"),
  "Navigator must defer row focus while background until mouse-up (AIA-266)",
);
assert.ok(
  nav.includes("event.preventDefault()") || nav.includes("preventDefault()"),
  "background row mousedown must preventDefault to block tabindex focus raise",
);

console.log(
  "File drag-out checks OK (AIA-240 COPY|MOVE|LINK + AIA-261/264/266 Explorer press→drag).",
);
