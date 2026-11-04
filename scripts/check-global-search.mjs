// SPDX-License-Identifier: GPL-3.0-or-later
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const discovery = readFileSync(join(root, "src/views/DiscoveryView.vue"), "utf8");

for (const [label, text, needle] of [
  ["AppShell", shell, "discovery.globalSearch"],
  ["AppShell", shell, "requestGlobalSearchFocus"],
  ["AppShell", shell, "dispatchShortcutEvent"],
  ["DiscoveryView", discovery, "discovery-global-search"],
  ["DiscoveryView", discovery, "consumeGlobalSearchFocus"],
]) {
  if (!text.includes(needle)) {
    console.error(`${label} missing ${needle}`);
    process.exit(1);
  }
}

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/global-search-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
