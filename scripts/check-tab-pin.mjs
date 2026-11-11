// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const zh = readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");

if (!shell.includes("onToggleTabPin") || !shell.includes("@dblclick.stop=\"onToggleTabPin(tab.id)\"")) {
  console.error("tab strip must toggle pin via double-click");
  process.exit(1);
}
// FIL-49: no dedicated pin/unpin control on each tab (status glyph only when pinned).
const pinBtnOpen = /<button[^>]*class="[^"]*tab-pin[^"]*"/;
if (pinBtnOpen.test(shell) || /class="tab-pin"[\s\S]{0,400}@click\.stop="onToggleTabPin/.test(shell)) {
  console.error("tab strip must not expose a clickable pin button");
  process.exit(1);
}
if (!shell.includes('v-if="isTabPinned(tab.id)"') || !shell.includes('class="tab-pin"')) {
  console.error("pinned status glyph must render only when the tab is pinned");
  process.exit(1);
}
// FIL-31: pin glyph always uses thumbtack; never star (favorites keep star).
const pinBtnIdx = shell.indexOf('class="tab-pin"');
const pinBtnSlice = pinBtnIdx >= 0 ? shell.slice(pinBtnIdx, pinBtnIdx + 700) : "";
const hasPinGlyph =
  /<FluentIcon[^>]*name="pin"/.test(pinBtnSlice) ||
  /<FluentIcon[^>]*name='pin'/.test(pinBtnSlice) ||
  pinBtnSlice.includes('name="pin"');
const pinUsesStar =
  pinBtnSlice.includes("? 'pin' : 'star'") ||
  pinBtnSlice.includes('? "pin" : "star"') ||
  /tab-pin[\s\S]*?name="star"/.test(pinBtnSlice);
if (!shell.includes("tab--pinned") || !hasPinGlyph || pinUsesStar) {
  console.error("pinned tabs need tab--pinned chrome and a pin (not star) glyph");
  process.exit(1);
}
const model = readFileSync(join(root, "src/state/openState/model.ts"), "utf8");
if (!model.includes("safeOpenStateIndex")) {
  console.error("openState/model must sanitize indices (safeOpenStateIndex)");
  process.exit(1);
}
if (
  !/currentTabGroupIndex:\s*Math\.max\(\s*0,\s*safeOpenStateIndex\(\s*state\.currentTabGroupIndex\s*\)\s*-\s*removedThroughActive/.test(
    model,
  )
) {
  console.error("openStateWithoutPinned must floor currentTabGroupIndex at 0");
  process.exit(1);
}
const tabPinCssIdx = shell.indexOf(".tab-pin {");
const tabPinCss = tabPinCssIdx >= 0 ? shell.slice(tabPinCssIdx, tabPinCssIdx + 450) : "";
if (!shell.includes("border-left: 3px solid") || !tabPinCss.includes("pointer-events: none")) {
  console.error("pinned tabs need durable left accent + non-interactive pin status glyph (AIA-245 / FIL-49)");
  process.exit(1);
}
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const session = readFileSync(join(root, "src/state/openState/session.ts"), "utf8");
const persistIdx = session.indexOf("export async function persistOpenStateNow");
const persistSlice = persistIdx >= 0 ? session.slice(persistIdx, persistIdx + 500) : "";
if (
  !persistSlice.includes("work_groups_save_open_state") ||
  !persistSlice.includes("snapshotLeavingOpenState()") ||
  persistSlice.includes("snapshotLiveOpenState()")
) {
  console.error("openState session persist must strip pins via snapshotLeavingOpenState");
  process.exit(1);
}
const pins = readFileSync(join(root, "src/state/tabPins.ts"), "utf8");
if (
  !pins.includes("pickSinglePinnedId") ||
  !pins.includes("ensureSinglePinnedTab") ||
  !pins.includes("pinnedIdSet") ||
  !/pinnedTabIds\.value\s*=\s*isTabPinned\(id\)\s*\?\s*\[\]\s*:\s*\[id\]/.test(pins)
) {
  console.error("tabPins must enforce single-pin toggle (AIA-260)");
  process.exit(1);
}
if (!model.includes("collapsePinnedIds") || !browse.includes("pinnedIdSet(")) {
  console.error("OpenState pin merge/persist must collapse to a single pin id");
  process.exit(1);
}
if (!model.includes("ensureUniqueTabIds") || !model.includes("ensureUniqueTabIds({")) {
  console.error("normalizeRestoredOpenState must uniquify empty/duplicate tab ids (AIA-260)");
  process.exit(1);
}
const rustSync = readFileSync(join(root, "src-tauri/src/work_groups/sync.rs"), "utf8");
if (!rustSync.includes("ID_SEQ") || !rustSync.includes("ensure_member_tabs_assign_unique_ids")) {
  console.error("Rust tab id minting must be unique within a millisecond batch");
  process.exit(1);
}
if (!zh.includes("默认工作组")) {
  console.error("default work group label must be 默认工作组");
  process.exit(1);
}
if (!registry.includes("navigator.closeTab") || !registry.includes("navigator.jumpTab")) {
  console.error("missing tab close / jump shortcuts");
  process.exit(1);
}

const registerUrl = pathToFileURL(
  join(root, "scripts/register-ts-resolve.mjs"),
).href;
const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/tab-pin-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
