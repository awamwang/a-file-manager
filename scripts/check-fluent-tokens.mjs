// SPDX-License-Identifier: GPL-3.0-or-later
// Fluent chrome token + utility presence (S1 + AIA-28 polish).
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
  "--fluent-statusbar-height: 32px",
  "--fluent-sidenav-width",
  "--fluent-control-height-standard: 32px",
  "--fluent-list-row-height-standard: 32px",
  "--fluent-list-row-height-compact: 24px",
  "--fluent-font-family",
  "--fluent-color-fill-control-hover",
  "--fluent-color-fill-selected",
  "--fluent-color-status-danger",
  "--fluent-color-status-warning",
  "--fluent-color-status-info",
  "--fluent-color-status-success-bg",
  "--fluent-color-fill-layer-alt",
  "--fluent-color-stroke-accent-soft",
  "--fluent-color-chip-ok",
  "--fluent-color-icon-wash",
  "--fluent-color-stroke-focus",
  "--fluent-shadow-flyout",
  "--fluent-overlay-scrim",
  "html[data-theme=\"dark\"]",
  "html[data-theme=\"light\"]",
  "html[data-list-density=\"compact\"]",
  ".fluent-chrome",
  ".fluent-control",
  ".fluent-control--accent",
  ".fluent-list-row",
  ".fluent-list-row--density",
  ".fluent-page",
  ".fluent-banner",
  ".fluent-banner--error",
  ".fluent-banner--info",
  ".fluent-banner--warning",
  ".fluent-surface",
  ".fluent-chip",
  ".fluent-chip--ok",
  ".fluent-chip--down",
  ".fluent-icon-wash",
  ".fluent-layer-alt",
  ".fluent-field-check",
];

const missing = required.filter((token) => !css.includes(token));
if (missing.length) {
  console.error("Fluent token presence check failed. Missing:");
  for (const item of missing) console.error(`  - ${item}`);
  process.exit(1);
}

console.log(`Fluent token presence OK (${required.length} checks).`);
