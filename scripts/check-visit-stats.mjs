// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const rust = readFileSync(join(root, "src-tauri/src/visit_stats/mod.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(shell.includes("places-menu") || shell.includes("quickAccessList"));
assert.ok(shell.includes("quickAccessList"));
assert.ok(shell.includes("openPlacesPath"));
assert.ok(shell.includes("loadVisitStats"));
assert.ok(browse.includes("recordPathVisit"));
assert.ok(rust.includes("visit-stats.json"));
assert.ok(rust.includes("VISIT_STATS_FILE_NAME"));
assert.ok(
  rust.includes("Independent of Work Group"),
  "visit-stats must document independence from WG openState",
);
assert.ok(!rust.includes("work_groups"), "visit-stats must not touch WG store");
assert.ok(lib.includes("visit_stats::visit_stats_record"));
assert.equal(en.quickAccess.label, "Recent visits");
assert.ok(en.quickAccess.openCount.includes("{count}"));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/visit-stats-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "visit stats ranking checks failed");

console.log("Visit stats / Quick Access checks OK (AIA-74).");
