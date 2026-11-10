// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const list = readFileSync(join(root, "src-tauri/src/navigator/list.rs"), "utf8");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");

if (!list.includes("entry_is_hidden") || !list.includes("FILE_ATTRIBUTE_HIDDEN")) {
  console.error("listing must skip FILE_ATTRIBUTE_HIDDEN");
  process.exit(1);
}
if (!browse.includes("showHidden: showHiddenItems.value")) {
  console.error("navigator list must pass showHidden");
  process.exit(1);
}
if (!shell.includes("chrome.showHidden") || !shell.includes("onToggleShowHidden")) {
  console.error("navigator chrome needs a show-hidden toggle");
  process.exit(1);
}
// FIL-31: show-hidden uses eye / eyeOff; view-mode trigger uses list — not the same glyph.
const glyphs = readFileSync(join(root, "src/components/icons/glyphs.ts"), "utf8");
if (!shell.includes("showHiddenItems ? 'eye' : 'eyeOff'") && !shell.includes('showHiddenItems ? "eye" : "eyeOff"')) {
  console.error("show-hidden toggle must use eye / eyeOff glyphs");
  process.exit(1);
}
const viewModeIdx = shell.indexOf('id="navigator-view-mode-menu"');
const viewModeSlice = viewModeIdx >= 0 ? shell.slice(viewModeIdx, viewModeIdx + 600) : "";
if (!viewModeSlice.includes('name="list"') && !viewModeSlice.includes("name='list'")) {
  console.error("view-mode trigger must use list glyph (distinct from eye)");
  process.exit(1);
}
if (!glyphs.includes("list:") || !glyphs.includes("eyeOff:") || !glyphs.includes("eye:")) {
  console.error("glyphs catalog must define list, eye, and eyeOff");
  process.exit(1);
}
const listBlock = glyphs.slice(glyphs.indexOf("list:"), glyphs.indexOf("eye:"));
if (!listBlock.includes("\\uEA37")) {
  console.error("list glyph should use Segoe List U+EA37");
  process.exit(1);
}
if (listBlock.includes("\\uE7B3") || listBlock.includes("\\uE894")) {
  console.error("list glyph must not reuse eye / eyeOff Segoe codepoints");
  process.exit(1);
}
if (!defaults.includes("json!(false)") || !defaults.includes("core.showHidden")) {
  console.error("core.showHidden must default off");
  process.exit(1);
}

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/show-hidden-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
