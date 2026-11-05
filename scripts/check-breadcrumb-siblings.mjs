// SPDX-License-Identifier: GPL-3.0-or-later
// AIA: Windows-style breadcrumb sibling dropdown.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(shell.includes("toggleCrumbMenu"));
assert.ok(shell.includes("onCrumbSibling"));
assert.ok(shell.includes("navigator-crumb-siblings") || shell.includes("crumb-menu"));
assert.ok(shell.includes("crumb-chevron"));
assert.ok(shell.includes('navigator_list'));
assert.ok(/toggleCrumbMenu[\s\S]*?navigator_list/.test(shell));
assert.ok(/onCrumbSibling[\s\S]*?navigateTo/.test(shell));
assert.equal(en.chrome.crumbSiblings, "Sibling folders");
assert.ok(en.chrome.crumbSiblingsEmpty);
assert.ok(en.chrome.crumbSiblingsLoading);

console.log("breadcrumb siblings checks OK.");
