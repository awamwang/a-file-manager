// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const mode = readFileSync(join(root, "src/state/navigatorViewMode.ts"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(mode.includes('"tiles"'));
assert.ok(mode.includes('"content"'));
assert.ok(mode.includes("navigatorGroupByType"));
assert.ok(mode.includes('"iconsExtraLarge"'), "AIA-233: Extra large mode");
assert.ok(mode.includes("iconsExtraLarge: 256") || mode.includes("iconsExtraLarge:256"));
assert.ok(view.includes("col-type"));
assert.ok(view.includes("onColumnResizeStart"));
assert.ok(view.includes("list--tiles"));
assert.ok(view.includes("list--content"));
assert.ok(view.includes("list--icons-extra-large"), "AIA-233 CSS class");
// 9723a38 窗口化重构：groupedVisibleEntries 改名为 primaryVirtItems
// （分组时经 flattenGroupedItems 打平），契约改为守护新的打平数据源。
assert.ok(view.includes("primaryVirtItems"));
assert.ok(view.includes("flattenGroupedItems"));
assert.ok(view.includes("onListWheel"), "AIA-232: Ctrl+wheel must be wired on the file list");
assert.ok(view.includes("nudgeIconViewMode"), "AIA-232: wheel uses icon zoom ladder");
assert.ok(mode.includes("ICON_VIEW_ZOOM_ORDER"));
assert.ok(mode.includes("nudgeIconViewMode"));
assert.ok(shell.includes("viewGroupByType"));
assert.ok(defaults.includes("tiles"));
assert.ok(defaults.includes("iconsExtraLarge"));
assert.ok(defaults.includes("core.navigatorGroupByType"));
assert.equal(en.navigator.colType, "Type");
assert.equal(en.chrome.viewTiles, "Tiles");
assert.equal(en.chrome.viewIconsExtraLarge, "Extra large icons");
assert.equal(en.chrome.viewIconsLarge, "Large icons");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/view-layout-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("View layout shell checks OK (AIA-191).");
