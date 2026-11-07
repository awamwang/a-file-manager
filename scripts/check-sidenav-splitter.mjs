// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-237: draggable divider between folder-tree sidenav and browse workspace.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const state = readFileSync(join(root, "src/state/sidenavWidth.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const tokens = readFileSync(join(root, "src/styles/fluent-tokens.css"), "utf8");

assert.ok(shell.includes('class="sidenav-splitter"'), "splitter element required");
assert.ok(shell.includes("onSidenavResizeStart"), "pointer drag start required");
assert.ok(shell.includes("onSidenavResizeMove"), "pointer drag move required");
assert.ok(shell.includes("applySidenavWidth"), "live drag applies without thrashing storage");
assert.ok(shell.includes("onSidenavSplitterKey"), "keyboard resize required");
assert.ok(shell.includes("loadSidenavWidth"), "persist hydrate on mount");
assert.ok(shell.includes("setSidenavWidth"), "width setter wired");
assert.ok(shell.includes(":style=\"{ width: `${sidenavWidth}px` }\""), "inline width binding");
assert.ok(shell.includes('role="separator"'), "a11y separator role");
assert.ok(shell.includes("chrome.resizeSideNav"), "resize aria label");

const sidenav = shell.indexOf('class="sidenav');
const splitter = shell.indexOf('class="sidenav-splitter"');
const workspace = shell.indexOf('class="workspace"');
assert.ok(sidenav >= 0 && splitter > sidenav && workspace > splitter, "order: sidenav → splitter → workspace");

assert.ok(state.includes("STORAGE_KEY") || state.includes("afm.sidenavWidth"));
assert.ok(state.includes("clampSidenavWidth"));
assert.ok(state.includes("writeStoredSidenavWidth"));
assert.ok(state.includes("MIN_SIDENAV_WIDTH"));
assert.ok(state.includes("MAX_SIDENAV_WIDTH"));
assert.equal(en.chrome.resizeSideNav, "Resize navigation pane");
assert.ok(tokens.includes("--fluent-sidenav-width: 200px"), "token default remains 200px");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/sidenav-width-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);

console.log("sidenav-splitter checks OK (AIA-237).");
