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
const mainWindow = JSON.parse(conf).app?.windows?.[0];
if (mainWindow?.decorations !== false) {
  console.error("main window must use decorations:false for custom Fluent caption");
  process.exit(1);
}
if (mainWindow?.maximized !== true) {
  console.error("main window must start maximized (not exclusive fullscreen)");
  process.exit(1);
}
if (mainWindow?.visible !== false) {
  console.error("main window must start visible:false (AIA-272: show after ready)");
  process.exit(1);
}
if (mainWindow?.backgroundColor !== "#f3f3f3") {
  console.error("main window must set Fluent fill-subtle backgroundColor (AIA-272)");
  process.exit(1);
}
if (mainWindow?.fullscreen === true) {
  console.error("main window must not use exclusive fullscreen on launch");
  process.exit(1);
}

for (const perm of [
  "core:window:allow-minimize",
  "core:window:allow-maximize",
  "core:window:allow-toggle-maximize",
  "core:window:allow-close",
  "core:window:allow-show",
  "core:window:allow-start-dragging",
  "core:window:allow-unminimize",
]) {
  if (!caps.includes(perm)) {
    console.error(`capabilities missing ${perm}`);
    process.exit(1);
  }
}

const mainTs = readFileSync(join(root, "src/main.ts"), "utf8");
const reveal = readFileSync(join(root, "src/state/revealMainWindow.ts"), "utf8");
if (!mainTs.includes("revealMainWindowAfterReady")) {
  console.error("main.ts must call revealMainWindowAfterReady after mount (AIA-272)");
  process.exit(1);
}
if (
  !reveal.includes("afm_startup_should_reveal") ||
  !reveal.includes("afm_reveal_main_window") ||
  !reveal.includes(".maximize()")
) {
  console.error("revealMainWindow must gate on afm_startup_should_reveal, maximize, then host reveal");
  process.exit(1);
}
if (!libRs.includes("afm_startup_should_reveal") || !libRs.includes("afm_reveal_main_window") || !libRs.includes("StartupReveal")) {
  console.error("lib.rs must register StartupReveal + both startup reveal commands (AIA-272/FIL-72)");
  process.exit(1);
}
if (!indexHtml.includes('style="background:#f3f3f3"')) {
  console.error("index.html must set early #f3f3f3 background before CSS (AIA-272)");
  process.exit(1);
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

// 7.18/7.19: macOS Overlay chrome is composed at window creation and finished
// in the platform adapter — no AppKit details may leak back into lib.rs. The
// adapter restores FullSizeContentView and centres the traffic lights from
// their measured window coordinates instead of a hard-coded inset.
const macOsAdapter = readFileSync(join(root, "src-tauri/src/platform/macos.rs"), "utf8");
const platformMod = readFileSync(join(root, "src-tauri/src/platform/mod.rs"), "utf8");
const cargoToml = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const fluentTokens = readFileSync(join(root, "src/styles/fluent-tokens.css"), "utf8");
if (mainWindow?.titleBarStyle !== "Overlay" || mainWindow?.hiddenTitle !== true) {
  console.error("main window needs titleBarStyle:Overlay + hiddenTitle for macOS traffic lights (7.17)");
  process.exit(1);
}
if (!libRs.includes("platform::extend_content_under_titlebar") || libRs.includes("objc2_app_kit")) {
  console.error("window-chrome geometry must delegate to platform adapter, not AppKit in lib.rs (7.18)");
  process.exit(1);
}
if (
  !macOsAdapter.includes("NSWindowStyleMask::FullSizeContentView") ||
  !macOsAdapter.includes("setTitlebarAppearsTransparent") ||
  macOsAdapter.includes("TRAFFIC_LIGHT_INSET_PT")
) {
  console.error("macOS adapter must restore FullSizeContentView without the old container inset hack (7.18)");
  process.exit(1);
}
if (
  !macOsAdapter.includes("center_traffic_lights_on_caption") ||
  !macOsAdapter.includes("convertRect_toView") ||
  !macOsAdapter.includes("const CAPTION_HEIGHT_PT: f64 = 40.0") ||
  !fluentTokens.includes("--fluent-titlebar-height: 40px")
) {
  console.error("traffic lights must be centred from window coordinates, with the caption height synced to CSS (7.19)");
  process.exit(1);
}
if (!platformMod.includes("pub fn extend_content_under_titlebar")) {
  console.error("platform/mod.rs must expose extend_content_under_titlebar (7.18)");
  process.exit(1);
}
if (!shell.includes("mac-traffic-light-gutter") || !shell.includes("flex: 0 0 78px")) {
  console.error("macOS caption needs a draggable 78px gutter below the traffic lights (7.18)");
  process.exit(1);
}
if (
  !cargoToml.includes("[target.'cfg(target_os = \"macos\")'.dependencies]") ||
  !cargoToml.includes("objc2-app-kit")
) {
  console.error("AppKit bindings must stay target-gated in Cargo.toml (7.17)");
  process.exit(1);
}

console.log("window-controls: ok");
