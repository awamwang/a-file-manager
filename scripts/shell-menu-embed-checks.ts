// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  flattenShellItems,
  shellEmbedLoadingPlaceholder,
} from "../src/components/navigator/shellMenuEmbed.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const host = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const embed = readFileSync(
  join(root, "src/components/navigator/shellMenuEmbed.ts"),
  "utf8",
);

const flat = flattenShellItems([
  { id: 10, name: "Open", verb: "open", children: null },
  {
    id: 0,
    name: "Send to",
    verb: null,
    children: [
      { id: 20, name: "Bluetooth", verb: null, children: null },
      { id: 21, name: "Desktop", verb: "link", children: null },
    ],
  },
  { id: 0, name: "Empty parent", verb: null, children: [] },
  { id: 0, name: "No id no verb", verb: null, children: null },
]);

assert.deepEqual(flat, [
  { commandId: 10, verb: "open", label: "Open" },
  { commandId: 20, verb: null, label: "Send to › Bluetooth" },
  { commandId: 21, verb: "link", label: "Send to › Desktop" },
]);

assert.deepEqual(flattenShellItems([]), []);

const loading = shellEmbedLoadingPlaceholder("Loading…");
assert.equal(loading.length, 1);
assert.equal(loading[0]?.id, "shell:loading");
assert.equal(loading[0]?.enabled, false);

assert.ok(embed.includes("export async function loadShellEmbedChildren"));
assert.ok(embed.includes("export async function invokeShellEmbedChild"));
assert.ok(host.includes("loadShellEmbedChildren"));
assert.ok(host.includes("invokeShellEmbedChild"));
assert.ok(!host.includes("flattenShellItems"));
assert.ok(!host.includes("shell_get_context_menu"));
assert.ok(!host.includes("shell_invoke_context_menu_item"));

console.log("Shell menu embed checks OK (AIA-239).");
