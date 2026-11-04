// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(nav.includes("NavigatorContextMenu"));
assert.ok(nav.includes("@contextmenu"));
assert.ok(menu.includes("copyPath"));
assert.ok(menu.includes("revealItemInDir"));
assert.ok(menu.includes("newNavigatorTab"));
assert.ok(menu.includes("deleteStub") || menu.includes("contextMenu.deleteStub"));
assert.equal(en.contextMenu.copyPath, "Copy path");

for (const id of ["open", "openInNewTab", "reveal", "copyPath", "delete"]) {
  assert.ok(menu.includes(`"${id}"`) || menu.includes(`'${id}'`), `missing action ${id}`);
}

console.log("Context menu checks OK (Navigator Day-1 set includes Copy path).");
