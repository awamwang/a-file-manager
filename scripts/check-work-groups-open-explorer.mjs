// SPDX-License-Identifier: GPL-3.0-or-later
// W1 baseline: Work Groups must keep an "open in the OS file manager" (Explorer) entry.
// This mirrors check-open-project-folder.mjs for the WorkGroups side, which previously
// had NO gate. The real path-forwarding is covered by the cargo test in open_with
// (EXPLORER_LAST_PATH) and runs under test:rust; this gate pins the FE wiring contract:
// the WorkGroups view invokes open_path_default for the selected path.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(
  view.includes("async function openPathInSystemExplorer"),
  "WorkGroups view must expose openPathInSystemExplorer",
);
assert.ok(
  view.includes('"open_path_default"'),
  "WorkGroups reveal must invoke open_path_default",
);
assert.ok(
  view.includes("const text = path.trim();") && view.includes("if (!text) return;"),
  "WorkGroups reveal must guard an empty path before invoking",
);
assert.ok(
  view.includes("noticeOpenedInSystemExplorer"),
  "WorkGroups reveal must surface an opened-in-explorer notice",
);
assert.ok(
  typeof en.projects?.noticeOpenedInSystemExplorer === "string" &&
    en.projects.noticeOpenedInSystemExplorer.length > 0,
  "en.projects.noticeOpenedInSystemExplorer must exist (shared with ProjectsView)",
);

console.log("work-groups-open-explorer (W1) OK: WorkGroups view wires open_path_default for reveal.");
