// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const checks = [
  // ADR-0023: ExtensionsView is now the unified section hub (glyphs bound via
  // icon: "…" / :name="section.icon"); section-title depth renders in sections.
  ["src/views/ExtensionsView.vue", ['icon: "puzzle"', 'icon: "cloud"', "STATIC_SECTIONS", "ext-nav"]],
  ["src/components/extensions/ExtensionsInstalledSection.vue", ["section-title", 'name="library"']],
  ["src/views/WorkGroupsView.vue", ["group-head", "viewAll", "'home'"]],
  [
    "src/views/ProjectsView.vue",
    ["group-head", "fluent-surface", 'name="board"'],
  ],
];

for (const [file, needles] of checks) {
  const text = readFileSync(join(root, file), "utf8");
  for (const needle of needles) {
    if (!text.includes(needle)) {
      console.error(`P4 depth check failed: ${file} missing ${needle}`);
      process.exit(1);
    }
  }
}

console.log(`Fluent P4 content depth OK (${checks.length} views).`);
