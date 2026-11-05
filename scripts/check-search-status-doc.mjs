// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-84: search inventory doc present with required sections.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const docPath = join(root, "docs/a-file-manager-search-status.md");
let text;
try {
  text = readFileSync(docPath, "utf8");
} catch {
  console.error("missing docs/a-file-manager-search-status.md");
  process.exit(1);
}

for (const needle of [
  "AIA-84",
  "Everything",
  "blacklist",
  "Ctrl+Shift+F",
  "fallback",
  "indexState",
  "讨论",
]) {
  if (!text.includes(needle)) {
    console.error(`search-status doc missing ${needle}`);
    process.exit(1);
  }
}

console.log("search-status-doc: ok");
