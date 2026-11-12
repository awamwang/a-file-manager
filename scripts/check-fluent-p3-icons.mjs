// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const checks = [
  ["src/views/NavigatorView.vue", ["ShellIconImg"]],
  ["src/views/DiscoveryView.vue", ["entryKindIcon", 'name="search"', "FluentIcon"]],
  ["src/views/ProvidersView.vue", ["entryKindIcon", "FluentIcon"]],
  ["src/components/command/CommandPanel.vue", ["commandGroupIcon", "cmd-item-icon"]],
  ["src/views/SettingsView.vue", ["settingsIconForKey", "fluent-icon-wash"]],
];

for (const [file, needles] of checks) {
  const text = readFileSync(join(root, file), "utf8");
  for (const needle of needles) {
    if (!text.includes(needle)) {
      console.error(`P3 icons check failed: ${file} missing ${needle}`);
      process.exit(1);
    }
  }
}

console.log(`Fluent P3 list/settings/command icons OK (${checks.length} files).`);
