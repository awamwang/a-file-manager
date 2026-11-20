// SPDX-License-Identifier: GPL-3.0-or-later
// P2: Discovery/Providers/WG/Projects/Extensions use shared fluent page primitives.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const files = [
  "src/views/DiscoveryView.vue",
  "src/views/ProvidersView.vue",
  "src/views/WorkGroupsView.vue",
  "src/views/ProjectsView.vue",
];

const required = ["fluent-page", "fluent-page-header", "fluent-page-title", "fluent-banner"];

for (const file of files) {
  const text = readFileSync(join(root, file), "utf8");
  for (const needle of required) {
    if (!text.includes(needle)) {
      console.error(`P2 primitives check failed: ${file} missing ${needle}`);
      process.exit(1);
    }
  }
  // Local page chrome should not reinvent panel/header/h1 title stacks.
  if (/<section class="panel">/.test(text)) {
    console.error(`P2 primitives check failed: ${file} still uses class="panel"`);
    process.exit(1);
  }
  if (/<header class="header">/.test(text)) {
    console.error(`P2 primitives check failed: ${file} still uses class="header"`);
    process.exit(1);
  }
}

// ADR-0023: ExtensionsView is a unified section hub — it owns the page primitives
// and surface, while the shared banner primitive renders inside its sections.
{
  const text = readFileSync(join(root, "src/views/ExtensionsView.vue"), "utf8");
  for (const needle of ["fluent-page", "fluent-page-header", "fluent-page-title", "fluent-banner"]) {
    const source = needle === "fluent-banner"
      ? readFileSync(join(root, "src/components/extensions/ExtensionsInstalledSection.vue"), "utf8")
      : text;
    if (!source.includes(needle)) {
      console.error(`P2 primitives check failed: ExtensionsView (${needle}) missing`);
      process.exit(1);
    }
  }
}

const surfaceFiles = [
  "src/views/DiscoveryView.vue",
  "src/views/ProvidersView.vue",
  "src/views/WorkGroupsView.vue",
  "src/views/ProjectsView.vue",
  "src/views/ExtensionsView.vue",
];
for (const file of surfaceFiles) {
  const text = readFileSync(join(root, file), "utf8");
  if (!text.includes("fluent-surface")) {
    console.error(`P2 primitives check failed: ${file} missing fluent-surface`);
    process.exit(1);
  }
}

console.log(`Fluent P2 page primitives OK (${files.length} views).`);
