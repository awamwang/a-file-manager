// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const state = readFileSync(join(root, "src/state/dualPane.ts"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(state.includes("toggleDualPane"));
assert.ok(state.includes("navigateSecondary"));
assert.ok(view.includes("panes--dual"));
assert.ok(view.includes("dualPaneEnabled"));
assert.ok(view.includes("secondaryEntries"));
assert.ok(shell.includes("toggleDualPane"));
assert.ok(shell.includes("chrome.dualPane") || shell.includes('t("chrome.dualPane")'));
assert.equal(en.chrome.dualPane, "Dual pane");

console.log("Dual pane checks OK (AIA-68).");
