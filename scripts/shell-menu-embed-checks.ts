// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { flattenShellItems } from "../src/components/navigator/shellMenuEmbed.ts";

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

console.log("Shell menu embed checks OK (AIA-239).");
