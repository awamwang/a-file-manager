// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-175: native OLE/file drag-out to external apps (players, Explorer, …).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const caps = readFileSync(join(root, "src-tauri/capabilities/default.json"), "utf8");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const dragOut = readFileSync(join(root, "src/state/fileDragOut.ts"), "utf8");
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");

assert.ok(
  cargo.includes("tauri-plugin-drag"),
  "Cargo.toml must depend on tauri-plugin-drag (Windows OLE DoDragDrop)",
);
assert.ok(
  lib.includes("tauri_plugin_drag::init()"),
  "lib.rs must register tauri_plugin_drag",
);
assert.ok(caps.includes('"drag:default"'), "capabilities must grant drag:default");
assert.ok(
  pkg.dependencies?.["@crabnebula/tauri-plugin-drag"],
  "package.json must depend on @crabnebula/tauri-plugin-drag",
);

assert.ok(dragOut.includes("startDrag"), "fileDragOut must call startDrag");
assert.ok(
  dragOut.includes('@crabnebula/tauri-plugin-drag'),
  "fileDragOut must import CrabNebula drag plugin",
);
assert.ok(dragOut.includes('mode: "copy"'), "outbound drag must use copy mode");
assert.ok(
  dragOut.includes("armNativeFileDragOut"),
  "fileDragOut must export armNativeFileDragOut",
);
assert.ok(
  dragOut.includes("isOutsideViewport") || dragOut.includes("innerWidth"),
  "must detect leave-window to start OLE drag-out",
);
assert.ok(
  dragOut.includes("resolveResource"),
  "must resolve bundled drag preview icon",
);

assert.ok(
  nav.includes("armNativeFileDragOut"),
  "NavigatorView must arm native drag-out on dragstart",
);
assert.ok(
  nav.includes('application/x-afm-paths'),
  "NavigatorView must keep in-app HTML5 path payload",
);
assert.ok(
  nav.includes("copyMove") || nav.includes('effectAllowed = "copyMove"'),
  "dragstart should allow copy+move (OLE copy + in-app move)",
);

console.log("File drag-out checks OK (AIA-175 OLE / tauri-plugin-drag).");
