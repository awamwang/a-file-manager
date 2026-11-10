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
// FIL-31: favorites/places keep star; tab pin trigger must not.
const pinBlockStart = shell.indexOf('class="tab-pin"');
const pinBlock = pinBlockStart >= 0 ? shell.slice(pinBlockStart, pinBlockStart + 500) : "";
if (pinBlock.includes('name="star"') || pinBlock.includes("name='star'")) {
  console.error("tab pin trigger must not use star (favorites keep star)");
  process.exit(1);
}
if (!shell.includes('name="dualPane"')) {
  console.error("Dual Pane toggle must use dualPane glyph");
  process.exit(1);
}

// Places must not still use home as its trigger icon (home is dedicated).
const placesStart = shell.indexOf("navigator-places-menu");
const placesEnd = shell.indexOf("navigator-view-mode-menu");
if (placesStart < 0 || placesEnd < 0 || placesEnd <= placesStart) {
  console.error("cannot locate Places menu block before view-mode menu");
  process.exit(1);
}
const placesBlock = shell.slice(placesStart, placesEnd);
if (placesBlock.includes('name="home"')) {
  console.error("Places menu still uses home glyph as trigger");
  process.exit(1);
}

if (!glyphs.includes("star:") || !glyphs.includes("dualPane:")) {
  console.error("glyphs catalog missing star / dualPane");
  process.exit(1);
}

// Dual Pane must be its own glyph — not the page-group / viewAll 2×2 tile.
const dualPaneBlock = glyphs.slice(glyphs.indexOf("dualPane:"), glyphs.indexOf("add:"));
if (dualPaneBlock.includes("\\uE8A9")) {
  console.error("dualPane must not reuse viewAll Segoe codepoint U+E8A9");
  process.exit(1);
}
if (!dualPaneBlock.includes("7.55")) {
  // Dashed center column uses x≈7.55 segments in the SVG paths.
  console.error("dualPane SVG should emphasize a dashed vertical split");
  process.exit(1);
}
const dualBtnIdx = shell.indexOf('name="dualPane"');
if (dualBtnIdx < 0) {
  console.error("AppShell missing dualPane FluentIcon");
  process.exit(1);
}
const dualBtn = shell.slice(Math.max(0, dualBtnIdx - 100), dualBtnIdx + 60);
if (!dualBtn.includes("force-svg") && !dualBtn.includes("forceSvg")) {
  console.error("Dual Pane FluentIcon should force-svg for dashed split metaphor");
  process.exit(1);
}
if (en.chrome.home !== "Home") {
  console.error("en.chrome.home mismatch");
  process.exit(1);
}

console.log("navigator-icon-split: ok");
