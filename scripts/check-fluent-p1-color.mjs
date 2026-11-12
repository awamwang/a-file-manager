// SPDX-License-Identifier: GPL-3.0-or-later
// P1 semantic color wiring smoke.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function read(rel) {
  return readFileSync(join(root, rel), "utf8");
}

const checks = [
  ["src/views/DiscoveryView.vue", ["engine-card", "fluent-banner--info", "toastError"]],
  [
    "src/views/ProvidersView.vue",
    ["fluent-accent-edge--ok", "fluent-chip--down", "fluent-banner--error", "backend--down"],
  ],
  [
    "src/views/ExtensionsView.vue",
    ["fluent-chip--down", "fluent-banner--info", "state-chip", "card--warn"],
  ],
  [
    "src/views/HealthView.vue",
    ["fluent-banner--success", "fluent-banner--warning", "fluent-chip--ok", "fluent-chip--warn", "fluent-status-text--ok"],
  ],
  ["src/views/ProjectsView.vue", ["fluent-banner", "fluent-banner--error"]],
  ["src/components/shell/AppShell.vue", ["fluent-status-text--down", "fluent-status-text--ok"]],
  [
    "src/styles/fluent-tokens.css",
    [".fluent-banner--success", ".fluent-accent-edge--ok", ".fluent-status-text--warn"],
  ],
];

for (const [file, needles] of checks) {
  const text = read(file);
  for (const needle of needles) {
    if (!text.includes(needle)) {
      console.error(`P1 color check failed: ${file} missing ${needle}`);
      process.exit(1);
    }
  }
}

console.log(`Fluent P1 semantic color wiring OK (${checks.length} files).`);
