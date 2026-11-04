// SPDX-License-Identifier: GPL-3.0-or-later
// Minimal presence test for Fluent chrome tokens (S1).
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const css = readFileSync(join(root, "src/styles/fluent-tokens.css"), "utf8");

const required = [
  "--fluent-space-sm",
  "--fluent-space-md",
  "--fluent-space-lg",
  "--fluent-titlebar-height: 40px",
  "--fluent-titlebar-button-width: 46px",
  "--fluent-toolbar-height: 40px",
  "--fluent-addressbar-height: 36px",
  "--fluent-control-height-standard: 32px",
  "--fluent-list-row-height-standard: 32px",
  "--fluent-font-family",
  ".fluent-chrome",
  ".fluent-control",
  ".fluent-list-row",
];

const missing = required.filter((token) => !css.includes(token));
if (missing.length) {
  console.error("Fluent token presence check failed. Missing:");
  for (const item of missing) console.error(`  - ${item}`);
  process.exit(1);
}

console.log(`Fluent token presence OK (${required.length} checks).`);
