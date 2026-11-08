// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-272: no black window / maximize flash — visible:false + show after ready.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const conf = JSON.parse(read("src-tauri/tauri.conf.json"));
const mainWindow = conf.app?.windows?.[0];
const libRs = read("src-tauri/src/lib.rs");
const tray = read("src-tauri/src/system_tray.rs");
const mainTs = read("src/main.ts");
const reveal = read("src/state/revealMainWindow.ts");
const indexHtml = read("index.html");
const caps = read("src-tauri/capabilities/default.json");

assert.equal(mainWindow?.visible, false, "window must start hidden");
assert.equal(mainWindow?.maximized, true, "window must create already maximized (while hidden)");
assert.equal(mainWindow?.backgroundColor, "#f3f3f3", "Fluent fill-subtle webview/window bg");
assert.equal(mainWindow?.decorations, false);

assert.ok(indexHtml.includes('style="background:#f3f3f3"'), "html early bg");
assert.ok(indexHtml.includes('style="background:#f3f3f3;margin:0"'), "body early bg");

assert.ok(tray.includes("struct StartupReveal"));
assert.ok(tray.includes("afm_startup_should_reveal"));
assert.ok(libRs.includes("system_tray::afm_startup_should_reveal"));
assert.ok(libRs.includes("StartupReveal::new(!hide_on_autostart)"));
assert.ok(
  !libRs.includes("hide_main_window(app.handle())"),
  "setup must not show-then-hide; start invisible + reveal gate instead",
);

assert.ok(mainTs.includes("revealMainWindowAfterReady"));
assert.ok(reveal.includes("afm_startup_should_reveal"));
assert.ok(reveal.includes("requestAnimationFrame"));
assert.ok(reveal.includes(".maximize()"));
assert.ok(reveal.includes(".show()"));
assert.ok(reveal.includes(".setFocus()"));

assert.ok(caps.includes("core:window:allow-show"));
assert.ok(caps.includes("core:window:allow-maximize"));
assert.ok(caps.includes("core:window:allow-set-focus"));

console.log("startup-window-flash checks OK (AIA-272).");
