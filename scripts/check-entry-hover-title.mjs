// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-167: list-row hover title shows size (files) + modified time from listing fields.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");

for (const needle of [
  "function entryHoverTitle",
  "entryHoverTitle(entry)",
  "navigator.colSize",
  "navigator.colModified",
  "formatSize(entry)",
  "formatModified(entry)",
  "modifiedMs",
]) {
  if (!view.includes(needle)) {
    console.error(`NavigatorView missing hover-title piece: ${needle}`);
    process.exit(1);
  }
}

if (!view.includes(":title=\"entryHoverTitle(entry)\"")) {
  console.error("NavigatorView rows must bind :title=\"entryHoverTitle(entry)\"");
  process.exit(1);
}

if (view.includes("entry.kind === 'dir' ? t('navigator.folder') : t('navigator.file')")) {
  // Kind-only title on the icon span was replaced by the richer row tip.
  // Allow the same ternary inside entryHoverTitle (first line); ban only if it remains as a :title binding.
  const titleKindOnly =
    /:title="entry\.kind === 'dir' \? t\('navigator\.folder'\) : t\('navigator\.file'\)"/;
  if (titleKindOnly.test(view)) {
    console.error("NavigatorView still binds kind-only :title on list chrome");
    process.exit(1);
  }
}

// Folders must omit size in the hover tip (no child-count on listing entries).
const hoverFn = view.slice(view.indexOf("function entryHoverTitle"));
const hoverBody = hoverFn.slice(0, hoverFn.indexOf("\nfunction ") > 0 ? hoverFn.indexOf("\nfunction ") : 800);
if (!hoverBody.includes('entry.kind !== "dir"') && !hoverBody.includes("entry.kind !== 'dir'")) {
  console.error("entryHoverTitle must gate size to non-dir entries");
  process.exit(1);
}

console.log("entry-hover-title: ok");
