// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const state = readFileSync(join(root, "src/state/dualPane.ts"), "utf8");
const layout = readFileSync(join(root, "src/state/dualPaneLayout.ts"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const tauri = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));

assert.ok(state.includes("toggleDualPane"));
assert.ok(state.includes("navigateSecondary"));
assert.ok(state.includes("dualPaneLayout"), "re-exports layout clamp helpers");
assert.ok(layout.includes("DUAL_PANE_MIN_PANE_PX = 240"));
assert.ok(layout.includes("DUAL_PANE_WINDOW_MIN_WIDTH_PX"));
assert.ok(layout.includes("windowMinWidthForDualPane"));
assert.ok(layout.includes("dualPaneFitsInnerWidth"));
assert.ok(layout.includes("DEFAULT_WINDOW_MIN_WIDTH_PX = 330"));
assert.equal(tauri.app.windows[0].minWidth, 330, "baseline window minWidth stays 330");

assert.ok(view.includes("panes--dual"));
assert.ok(view.includes("dualPaneEnabled"));
assert.ok(view.includes("secondaryEntries"));
assert.ok(view.includes("min-width: 15rem"), "hard dual-pane pane floor");
assert.ok(view.includes("flex: 1 1 15rem"), "dual-pane flex basis matches floor");
assert.ok(!view.includes("min(12rem, 40%)"), "old self-defeating % clamp removed");
assert.ok(view.includes("overflow-x: auto"), "narrow host scrolls instead of crushing");

assert.ok(shell.includes("toggleDualPane"));
assert.ok(shell.includes("chrome.dualPane") || shell.includes('t("chrome.dualPane")'));
assert.ok(shell.includes("applyDualPaneWindowMinWidth"), "innerWidth / min-window guard");
assert.ok(shell.includes("setMinSize"), "Tauri window min size clamp");
assert.ok(shell.includes("windowMinWidthForDualPane"), "uses shared clamp helper");
assert.equal(en.chrome.dualPane, "Dual pane");

const runtime = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/dual-pane-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (runtime.stdout) process.stdout.write(runtime.stdout);
if (runtime.stderr) process.stderr.write(runtime.stderr);
if (runtime.status !== 0) process.exit(runtime.status ?? 1);

console.log("Dual pane checks OK (AIA-68/AIA-252).");
