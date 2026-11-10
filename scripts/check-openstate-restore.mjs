// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-100: flush Open State on close / hide so restart restores tabs.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const session = readFileSync(join(root, "src/state/openState/session.ts"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");

assert.ok(browse.includes("flushOpenStatePersist") || session.includes("flushOpenStatePersist"));
assert.ok(session.includes("snapshotLiveOpenState()"));
assert.ok(/bootstrapNavigatorOpenState[\s\S]*?flushOpenStatePersist/.test(browse));
assert.ok(shell.includes("flushOpenStatePersist"));
assert.ok(shell.includes("visibilitychange"));
assert.ok(/handleCloseRequested[\s\S]*?flushOpenStatePersist/.test(shell));
assert.ok(/applyCloseAction[\s\S]*?flushOpenStatePersist/.test(shell));

console.log("openstate restore checks OK (AIA-100).");
