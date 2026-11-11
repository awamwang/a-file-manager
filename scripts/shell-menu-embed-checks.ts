// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { showNativeShellContextMenu } from "../src/components/navigator/shellMenuEmbed.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const host = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const embed = readFileSync(
  join(root, "src/components/navigator/shellMenuEmbed.ts"),
  "utf8",
);

assert.equal(typeof showNativeShellContextMenu, "function");
assert.ok(embed.includes("export async function showNativeShellContextMenu"));
assert.ok(embed.includes("shell_show_context_menu"));
assert.ok(host.includes("showNativeShellContextMenu"));
assert.ok(host.includes("openShellCascadeFromAnchor"));
assert.ok(host.includes("shellCascade"));
assert.ok(!host.includes("loadShellEmbedChildren"));
assert.ok(!host.includes("invokeShellEmbedChild"));
assert.ok(!host.includes("shell_get_context_menu"));
assert.ok(!host.includes("shell_invoke_context_menu_item"));
assert.ok(!embed.includes("flattenShellItems"));

console.log("Shell menu native cascade checks OK (FIL-53).");
