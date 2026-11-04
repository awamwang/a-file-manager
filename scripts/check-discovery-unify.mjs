// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/DiscoveryView.vue"), "utf8");

assert.ok(view.includes("NavigatorContextMenu"));
assert.ok(view.includes("requestShellIcons"));
assert.ok(view.includes("hitToEntry"));
assert.ok(view.includes("onHitContext"));
assert.ok(view.includes("row--selected"));
assert.ok(view.includes("@keydown.enter"));

console.log("Discovery unify checks OK (AIA-69).");
