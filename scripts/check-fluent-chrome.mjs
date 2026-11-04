// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-28: chrome density contract + no Sigma kit leftovers in Vue styles.
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function walk(dir, out = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else if (/\.(vue|css|ts)$/.test(entry.name)) out.push(full);
  }
  return out;
}

const srcFiles = walk(join(root, "src"));
const banned = [/sigma-ui/i, /--sigma-/i, /infusion/i];
const hits = [];
for (const file of srcFiles) {
  const text = readFileSync(file, "utf8");
  for (const re of banned) {
    if (re.test(text)) hits.push(`${file} ~ ${re}`);
  }
}
if (hits.length) {
  console.error("Sigma / infusion leftovers found:");
  for (const hit of hits) console.error(`  - ${hit}`);
  process.exit(1);
}

const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
for (const needle of [
  "var(--fluent-statusbar-height)",
  "var(--fluent-sidenav-width)",
  "var(--fluent-color-fill-selected)",
  "var(--fluent-color-status-danger)",
]) {
  if (!shell.includes(needle)) {
    console.error(`AppShell missing chrome token usage: ${needle}`);
    process.exit(1);
  }
}

const uiChrome = join(root, "scripts", "ui-chrome-checks.ts");
const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", uiChrome],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);

console.log("Fluent chrome density / anti-Sigma check OK.");
