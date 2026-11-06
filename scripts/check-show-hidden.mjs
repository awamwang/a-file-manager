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
