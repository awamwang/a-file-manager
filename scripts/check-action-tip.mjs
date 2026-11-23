// SPDX-License-Identifier: GPL-3.0-or-later
// ADR 0025: floating shortcut hints must not appear from cold-start focus.
// Pure gate unit + surface wiring assertions (FE↔BE N/A: no Tauri command).
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const tip = read("src/shortcuts/actionTip.ts");
const host = read("src/components/common/ActionTipHost.vue");

// Surface: the directive gates only the focus trigger, not hover.
assert.ok(tip.includes('interactionGate.allows("focus")'));
assert.ok(tip.includes('el.addEventListener("focus", focus)'));
assert.ok(tip.includes("removeEventListener(\"focus\", focus)"));
assert.ok(tip.includes("export function markActionTipInteraction"));

// Surface: only real key/pointer input unlocks the gate. Scroll restoration
// and wheel events dismiss a tip but must not mark interaction.
assert.ok(host.includes("markActionTipInteraction"));
assert.ok(host.includes("window.addEventListener(\"pointerdown\", onUserInput, true)"));
assert.ok(host.includes("window.addEventListener(\"keydown\", onUserInput, true)"));
assert.ok(host.includes("window.addEventListener(\"scroll\", dismiss, true)"));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts", "action-tip-checks.ts")],
  { cwd: root, encoding: "utf8" },
);

if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
