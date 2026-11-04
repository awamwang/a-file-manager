// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(shell.includes("buildBreadcrumbs"));
assert.ok(shell.includes("onGoBack"));
assert.ok(shell.includes("onGoForward"));
assert.ok(shell.includes("class=\"breadcrumbs\"") || shell.includes("class='breadcrumbs'"));
assert.ok(shell.includes("canGoBack"));
assert.ok(!/toolbar-btn[\s\S]{0,80}historyLater/.test(shell));
assert.ok(registry.includes('id: "navigator.back"'));
assert.ok(registry.includes('id: "navigator.forward"'));
assert.ok(registry.includes('id: "navigator.backCtrlAlt"'));
assert.ok(registry.includes('id: "navigator.forwardCtrlAlt"'));
assert.ok(registry.includes("ctrl: true, alt: true, key: \"arrowleft\""));
assert.ok(registry.includes("ctrl: true, alt: true, key: \"arrowright\""));
assert.ok(shell.includes('registerShortcutHandler("navigator.backCtrlAlt"'));
assert.ok(shell.includes('registerShortcutHandler("navigator.forwardCtrlAlt"'));
assert.equal(en.chrome.breadcrumbs, "Breadcrumb path");
assert.equal(en.shortcuts.items.navigatorBack, "Navigate back");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/nav-history-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "nav history checks failed");

console.log("Nav history shell checks OK (AIA-63).");
