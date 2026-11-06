// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const zh = readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");

if (!shell.includes("onToggleTabPin") || !shell.includes("@dblclick.stop=\"onToggleTabPin(tab.id)\"")) {
  console.error("tab strip must pin from the button and a double-click");
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

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/tab-pin-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
