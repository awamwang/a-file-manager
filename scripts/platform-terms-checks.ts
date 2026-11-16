// SPDX-License-Identifier: GPL-3.0-or-later
/** Runtime checks for the platform copy merge (spawned by check-platform-terms.mjs). */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { localizeCatalog, macosOverrides } from "../src/i18n/platformTerms.ts";

type Catalog = Record<string, unknown>;

function lookup(catalog: Catalog, dotted: string): unknown {
  return dotted.split(".").reduce<unknown>((node, part) => {
    if (node && typeof node === "object" && part in (node as object)) {
      return (node as Catalog)[part];
    }
    return undefined;
  }, catalog);
}

const fixture: Catalog = {
  chrome: { commandsShortcut: "Ctrl+Shift+P" },
  guide: { title: "This PC" },
  platform: {
    macos: {
      "chrome.commandsShortcut": "⌘+⇧+P",
      "guide.title": "This Mac",
      "ghost.missing": "must not be created",
    },
  },
};
const snapshot = JSON.stringify(fixture);

assert.equal(macosOverrides(fixture)["chrome.commandsShortcut"], "⌘+⇧+P");
assert.deepEqual(macosOverrides({}), {});

const windows = localizeCatalog(fixture, false);
assert.equal(lookup(windows, "chrome.commandsShortcut"), "Ctrl+Shift+P");
assert.equal(lookup(windows, "guide.title"), "This PC");
assert.equal("platform" in windows, false, "internal section must not be translatable");

const macos = localizeCatalog(fixture, true);
assert.equal(lookup(macos, "chrome.commandsShortcut"), "⌘+⇧+P");
assert.equal(lookup(macos, "guide.title"), "This Mac");
assert.equal("ghost" in macos, false, "a broken override key must never create new copy");
assert.equal("platform" in macos, false);
assert.equal(JSON.stringify(fixture), snapshot, "raw catalog must not be mutated");

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
for (const id of ["en", "zh-CN", "ja", "de", "fr"]) {
  const raw = JSON.parse(
    readFileSync(join(root, `src/i18n/locales/${id}.json`), "utf8"),
  ) as Catalog;
  const overrides = (raw.platform as Catalog).macos as Record<string, string>;
  const merged = localizeCatalog(raw, true);
  for (const [key, value] of Object.entries(overrides)) {
    assert.equal(
      lookup(merged, key),
      value,
      `${id}:${key} macOS copy was not merged into the real catalog`,
    );
  }
  const base = localizeCatalog(raw, false);
  assert.equal(lookup(base, "chrome.commandsShortcut"), "Ctrl+Shift+P");
}

console.log("platform copy merge checks OK.");
