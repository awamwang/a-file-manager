// SPDX-License-Identifier: GPL-3.0-or-later
/** Guard: opt-in AFM_perf spans stay wired for baseline runs. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function read(rel) {
  return readFileSync(join(root, rel), "utf8");
}

const probe = read("src/state/perfProbe.ts");
assert.ok(probe.includes("afm.perfProbe"), "perfProbe reads localStorage flag");
assert.ok(probe.includes("[AFM_perf]"), "perfProbe logs AFM_perf prefix");
assert.ok(probe.includes("export async function perfSpan"), "perfSpan exported");
assert.ok(probe.includes("export function perfMark"), "perfMark exported");

const browse = read("src/state/navigatorBrowse.ts");
assert.ok(browse.includes('from "./perfProbe"'), "navigatorBrowse imports perfProbe");
assert.ok(/perfSpan\(\s*"navigator_list"/.test(browse), "navigator_list span");
assert.ok(/perfSpan\(\s*"tab_switch"/.test(browse), "tab_switch span");
assert.ok(browse.includes("revealPathForTabSwitch"), "tab switch uses timed reveal");
assert.ok(browse.includes("hydrateNavigatorListing"), "Phase C path hydrate");
assert.ok(browse.includes("joinListingChildPath"), "Phase C child path join");

const listRs = read("src-tauri/src/navigator/list.rs");
assert.ok(listRs.includes("skip_serializing_if"), "Phase C omit default entry path");
assert.ok(listRs.includes("elides_default_child_path_on_wire"), "Phase C elision test");

const icons = read("src/state/shellIcons.ts");
assert.ok(icons.includes('from "./perfProbe"'), "shellIcons imports perfProbe");
assert.ok(/perfSpan\(\s*"shell_icon_batch"/.test(icons), "shell_icon_batch span");

const tree = read("src/components/shell/FolderTreeBranch.vue");
assert.ok(tree.includes("perfProbe"), "FolderTreeBranch imports perfProbe");
assert.ok(/perfSpan\(\s*"tree_list"/.test(tree), "tree_list span");

const startup = read("src/state/startupTiming.ts");
assert.ok(startup.includes("afm.startupTiming"), "startupTiming flag preserved");
assert.ok(startup.includes("[AFM_startup]"), "startupTiming prefix preserved");

const perfLog = read("src-tauri/src/perf_log.rs");
assert.ok(perfLog.includes("AFM_PERF"), "Rust perf_log gated on AFM_PERF");
assert.ok(perfLog.includes("AFM_PERF_LOG"), "Rust perf_log supports file sink");

const navMod = read("src-tauri/src/navigator/mod.rs");
assert.ok(navMod.includes("afm_perf_line"), "navigator_list logs via perf_log");

const shellMod = read("src-tauri/src/shell_icons/mod.rs");
assert.ok(shellMod.includes("afm_perf_line"), "shell_icon_get_many logs via perf_log");

console.log("perf probe wiring checks OK");
