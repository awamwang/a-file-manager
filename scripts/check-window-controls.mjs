// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-80: custom titlebar system controls wiring (min / max / close / snap / drag).
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const indexHtml = readFileSync(join(root, "index.html"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const conf = readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8");
const caps = readFileSync(join(root, "src-tauri/capabilities/default.json"), "utf8");
const libRs = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const snap = readFileSync(join(root, "src-tauri/src/snap_chrome.rs"), "utf8");

if (!indexHtml.includes('id="afm-frame-boot"') || !indexHtml.includes("data-tauri-frame-tb")) {
  console.error("index.html missing afm-frame-boot sentinel for tauri-plugin-frame race");
  process.exit(1);
}
for (const id of ["frame-tb-minimize", "frame-tb-maximize", "frame-tb-close"]) {
  if (!indexHtml.includes(`id="${id}"`)) {
    console.error(`index.html boot sentinel missing ${id}`);
    process.exit(1);
  }
}

if (!JSON.parse(conf).app?.withGlobalTauri) {
  console.error("tauri.conf.json must set app.withGlobalTauri so frame controls.js can exit");
  process.exit(1);
}
if (JSON.parse(conf).app?.windows?.[0]?.decorations !== false) {
  console.error("main window must use decorations:false for custom Fluent caption");
  process.exit(1);
}

for (const perm of [
  "core:window:allow-minimize",
  "core:window:allow-maximize",
  "core:window:allow-toggle-maximize",
  "core:window:allow-close",
  "core:window:allow-start-dragging",
  "core:window:allow-unminimize",
]) {
  if (!caps.includes(perm)) {
    console.error(`capabilities missing ${perm}`);
    process.exit(1);
  }
}

for (const needle of [
  "reclaimFluentTitlebar",
  "onBeforeMount",
  "minimizeWindow",
  "toggleMaximizeWindow",
  "closeWindow",
  'id="frame-tb-minimize"',
  'id="frame-tb-maximize"',
  'id="frame-tb-close"',
  "data-tauri-drag-region",
  "tauri-frame://snap/click",
  "@dblclick.prevent=\"toggleMaximizeWindow\"",
  "app-region: no-drag",
]) {
  if (!shell.includes(needle)) {
    console.error(`AppShell missing window-chrome wiring: ${needle}`);
    process.exit(1);
  }
}

if (!libRs.includes("snap_overlay(true)") || !libRs.includes("create_overlay_titlebar_with_height")) {
  console.error("lib.rs missing frame snap overlay setup");
  process.exit(1);
}
if (!snap.includes("TITLEBAR_HEIGHT_PX") || !snap.includes("BUTTON_WIDTH_PX")) {
  console.error("snap_chrome.rs missing geometry constants");
  process.exit(1);
}

console.log("window-controls: ok");
