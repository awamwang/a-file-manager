// SPDX-License-Identifier: GPL-3.0-or-later
// Command Panel unit smoke (AIA-27): fuzzy filter + registry shape + shortcut helper.
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const runner = join(root, "scripts", "command-panel-checks.ts");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", runner],
  { cwd: root, encoding: "utf8" },
);

if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
