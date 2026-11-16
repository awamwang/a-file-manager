// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const glyphs = readFileSync(join(root, "src/components/icons/glyphs.ts"), "utf8");
const fluentIcon = readFileSync(join(root, "src/components/icons/FluentIcon.vue"), "utf8");
const shellIcon = readFileSync(join(root, "src/components/navigator/ShellIconImg.vue"), "utf8");
const navigatorView = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const entryIcon = readFileSync(join(root, "src/state/entryIcon.ts"), "utf8");
const providerListing = readFileSync(join(root, "src/state/provider/listing.ts"), "utf8");
const navigatorBrowse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const rustList = readFileSync(join(root, "src-tauri/src/navigator/list.rs"), "utf8");
const rustSearch = readFileSync(join(root, "src-tauri/src/navigator/search.rs"), "utf8");
const previewPane = readFileSync(join(root, "src/components/navigator/PreviewPane.vue"), "utf8");
const registerUrl = pathToFileURL(join(root, "scripts/register-ts-resolve.mjs")).href;

for (const id of ["shortcut", "bookmark", "provider"]) {
  assert.ok(glyphs.includes(`  ${id}: {`), `missing glyph ${id}`);
}
assert.equal(glyphs.match(/^  link: \{/gm)?.length, 1, "link glyph must be defined exactly once");
assert.ok(previewPane.includes('<FluentIcon name="link" size="lg" />'));
assert.ok(glyphs.includes('"\\uE71B"'), "PreviewPane link glyph codepoint must stay intact");
assert.ok(glyphs.includes("svgOnly?: boolean;"), "glyph catalog must support svgOnly");
assert.ok(glyphs.includes("AFM-internal ID"), "special glyphs need documented internal codepoints");
assert.ok(
  fluentIcon.indexOf("glyph.value.svgOnly") < fluentIcon.indexOf("/Windows/i.test"),
  "svgOnly must win before the Windows font preference",
);
// The grey slot stays the loading placeholder for ordinary local entries.
assert.ok(shellIcon.includes("shell-icon-slot"));
assert.ok(!shellIcon.includes("FluentIcon"), "ShellIconImg must not import a final glyph");

assert.ok(entryIcon.includes('from "./provider/path.ts"') || entryIcon.includes("./provider/path"));
assert.ok(entryIcon.includes('from "./recycle/path.ts"') || entryIcon.includes("./recycle/path"));
assert.ok(providerListing.includes("providerKind: providerKindFromExtra(entry.extra)"));
assert.ok(navigatorBrowse.includes("providerKind: entry.providerKind ?? null"));
// Symlink / junction flag travels from both native listings to row routing.
assert.ok(rustList.includes("pub is_link: bool"));
assert.ok(rustList.includes('skip_serializing_if = "is_false"'));
assert.ok(rustList.includes("file_type.is_symlink()"));
assert.ok(rustSearch.includes("entry.file_type().is_symlink()"));
assert.ok(navigatorBrowse.includes("isLink: entry.isLink ?? null"));
assert.ok(entryIcon.includes('if (entry.isLink) return "link";'));

// Both Navigator panes route special entries before enqueuing a shell icon.
assert.ok(navigatorView.includes('v-if="specialEntryIcon(item.entry, currentPath)"'));
assert.ok(navigatorView.includes('v-if="specialEntryIcon(entry, secondaryPath)"'));
assert.ok(navigatorView.includes("class=\"entry-special-icon\""));
assert.ok(navigatorView.includes(".kind :deep(.entry-special-icon)"));
assert.ok(navigatorView.includes("font-size: var(--nav-icon-size, 16px)"));

const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/entry-icon-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("Entry icon routing checks OK");
