// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-82: Home / star Places / dashed Dual Pane icons.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const glyphs = readFileSync(join(root, "src/components/icons/glyphs.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

if (!shell.includes('id="navigator-home-btn"')) {
  console.error("missing navigator-home-btn");
  process.exit(1);
}
if (!shell.includes("goHome") || !shell.includes("core.defaultLocation")) {
  console.error("Home must resolve core.defaultLocation then navigator_default_path");
  process.exit(1);
}
if (!shell.includes('name="star"')) {
  console.error("Places trigger must use star icon");
  process.exit(1);
}
if (!shell.includes('name="dualPane"')) {
  console.error("Dual Pane toggle must use dualPane glyph");
  process.exit(1);
}

// Places must not still use home as its trigger icon (home is dedicated).
const placesBlock = shell.slice(
  shell.indexOf("navigator-places-menu"),
  shell.indexOf("navigator-work-group-menu"),
);
if (placesBlock.includes('name="home"')) {
  console.error("Places menu still uses home glyph as trigger");
  process.exit(1);
}

if (!glyphs.includes("star:") || !glyphs.includes("dualPane:")) {
  console.error("glyphs catalog missing star / dualPane");
  process.exit(1);
}
if (en.chrome.home !== "Home") {
  console.error("en.chrome.home mismatch");
  process.exit(1);
}

console.log("navigator-icon-split: ok");
