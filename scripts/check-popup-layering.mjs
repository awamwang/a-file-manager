// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-255: overflow menus must Teleport out of overflow:hidden ancestors.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const wg = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
const projects = readFileSync(join(root, "src/views/ProjectsView.vue"), "utf8");
const tokens = readFileSync(join(root, "src/styles/fluent-tokens.css"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");

// Still true: surfaces clip; menus must leave them.
assert.match(tokens, /\.fluent-surface\s*\{[^}]*overflow:\s*hidden/s);

for (const [label, src] of [
  ["WorkGroupsView", wg],
  ["ProjectsView", projects],
]) {
  assert.ok(src.includes('<Teleport to="body">'), `${label}: Teleport to body`);
  assert.ok(src.includes("more-popover--portal"), `${label}: portal popover class`);
  assert.ok(src.includes("positionMorePopover"), `${label}: fixed positioning helper`);
  assert.ok(src.includes('zIndex: "1200"') || src.includes("zIndex: '1200'"), `${label}: portal z-index`);
  // List ⋯ popover must not be nested under the per-card list-more-menu anymore.
  const listMenuBlock = src.match(/class="list-more-menu"[\s\S]*?<\/div>/);
  assert.ok(listMenuBlock, `${label}: list-more-menu trigger host`);
  assert.ok(
    !listMenuBlock[0].includes("more-popover"),
    `${label}: list more-popover must not nest inside list-more-menu / card`,
  );
  assert.ok(src.includes("more-popover--portal"), `${label}: portal menus present`);
}

// Prior fix remains: breadcrumb siblings portal.
assert.ok(shell.includes("<Teleport to=\"body\">") || shell.includes("<Teleport to='body'>"));
assert.ok(shell.includes("positionCrumbMenu"));

console.log("popup layering (Teleport) checks OK.");
