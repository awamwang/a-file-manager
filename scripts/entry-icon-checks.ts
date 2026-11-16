// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/entry-icon-checks.ts

import assert from "node:assert/strict";
import { FLUENT_GLYPHS } from "../src/components/icons/glyphs.ts";
import { specialEntryIcon } from "../src/state/entryIcon.ts";
import {
  providerKindFromExtra,
  providerListingFromDir,
  providerRootEntriesFromBackends,
} from "../src/state/provider/listing.ts";
import { RECYCLE_PATH } from "../src/state/recycle/path.ts";

for (const id of ["shortcut", "bookmark", "provider"] as const) {
  const glyph = FLUENT_GLYPHS[id];
  assert.ok(glyph, `${id} missing from glyph catalog`);
  assert.equal(glyph.svgOnly, true, `${id} must force SVG on Windows`);
  assert.ok(glyph.paths.length >= 1, `${id} missing SVG path`);
}
// Symlinks reuse the existing Segoe link glyph shown in PreviewPane.
assert.equal(FLUENT_GLYPHS.link.codepoint, "\uE71B");
assert.notEqual(FLUENT_GLYPHS.link.svgOnly, true);
const specialIds = ["shortcut", "bookmark", "provider"];
const specialCodepoints = specialIds.map(
  (id) => FLUENT_GLYPHS[id as "shortcut"].codepoint,
);
assert.equal(new Set(specialCodepoints).size, specialIds.length);
for (const [id, glyph] of Object.entries(FLUENT_GLYPHS)) {
  if (specialCodepoints.includes(glyph.codepoint) && !specialIds.includes(id)) {
    assert.fail(`${id} collides with a special-entry codepoint`);
  }
}

// Local entries keep the shell-icon slot — only special kinds get a final glyph.
assert.equal(specialEntryIcon({ name: "Projects", path: "C:\\Work\\Projects", kind: "dir" }), null);
assert.equal(specialEntryIcon({ name: "notes.txt", path: "C:\\Work\\notes.txt", kind: "file" }), null);
assert.equal(
  specialEntryIcon({ name: "Edge.lnk", path: "C:\\Users\\Public\\Edge.lnk", kind: "file" }),
  "shortcut",
);
assert.equal(
  specialEntryIcon({ name: "EDGE.LNK", path: "C:\\Users\\Public\\EDGE.LNK", kind: "file" }),
  "shortcut",
);
assert.equal(
  specialEntryIcon({ name: "Shortcut", path: "C:\\Work\\Shortcut", kind: "dir" }),
  null,
);
// Filesystem links (symlink / junction) carry isLink from the native listing.
assert.equal(
  specialEntryIcon({
    name: "smart-review",
    path: "C:\\Users\\Administrator\\.cursor\\skills\\smart-review",
    kind: "dir",
    isLink: true,
  }),
  "link",
);
assert.equal(
  specialEntryIcon({ name: "alias.txt", path: "C:\\targets\\alias.txt", kind: "file", isLink: true }),
  "link",
);
assert.equal(
  specialEntryIcon({ name: "alias", path: "C:\\targets\\alias", kind: "dir", isLink: false }),
  null,
);
// Provider / recycle semantics win over the generic link glyph.
assert.equal(
  specialEntryIcon({
    name: "alias",
    path: "awam://knowledge/alias",
    kind: "dir",
    providerKind: "virtual",
    isLink: true,
  }),
  "provider",
);
assert.equal(
  specialEntryIcon({ name: "old", path: "C:\\old", kind: "dir", isLink: true }, RECYCLE_PATH),
  "recycle",
);

// Provider entries never reach the shell-icon queue (they have no native icon).
assert.equal(
  specialEntryIcon({
    name: "GitHub",
    path: "awam://browser/bookmarks/GitHub",
    kind: "file",
    providerKind: "bookmark",
  }),
  "bookmark",
);
assert.equal(
  specialEntryIcon({
    name: "Repo",
    path: "awam://browser/bookmarks/Repo",
    kind: "file",
    providerKind: " URL ",
  }),
  "bookmark",
);
assert.equal(
  specialEntryIcon({
    name: "Collections",
    path: "awam://knowledge/collections",
    kind: "dir",
    providerKind: "virtual",
  }),
  "provider",
);
assert.equal(
  specialEntryIcon({
    name: "Remote",
    path: "awam://rclone/remote",
    kind: "dir",
    providerKind: "backend",
  }),
  "provider",
);
assert.equal(
  specialEntryIcon({ name: "Root", path: "awam://root", kind: "dir" }),
  "provider",
);
// Provider child whose URI was rewritten to a local-looking path keeps its hint.
assert.equal(
  specialEntryIcon(
    { name: "Doc", path: "C:\\cache\\provider\\Doc", kind: "file", providerKind: "bookmark" },
    "awam://knowledge/space",
  ),
  "bookmark",
);
assert.equal(
  specialEntryIcon({ name: "Doc", path: "C:\\cache\\provider\\Doc", kind: "file" }, "awam://knowledge/space"),
  "provider",
);
// Provider-only markers must not hijack ordinary local rows.
assert.equal(
  specialEntryIcon({ name: "Doc", path: "C:\\Work\\Doc", kind: "file", providerKind: "bookmark" }),
  null,
);
// Recycle Bin items point at deleted originals; use the recycle glyph instead.
assert.equal(
  specialEntryIcon({ name: "old.txt", path: "C:\\Work\\old.txt", kind: "file" }, RECYCLE_PATH),
  "recycle",
);

for (const result of [
  specialEntryIcon({ name: "a.lnk", path: "C:\\a.lnk", kind: "file" }),
  specialEntryIcon({ name: "alias", path: "C:\\target", kind: "dir", isLink: true }),
  specialEntryIcon({ name: "b", path: "awam://browser/b", kind: "file" }),
  specialEntryIcon({ name: "c", path: "C:\\c", kind: "file" }, RECYCLE_PATH),
]) {
  assert.ok(result && FLUENT_GLYPHS[result], `unknown glyph ${result}`);
}

// Backend `extra` markers are reduced to one stable semantic hint.
assert.equal(providerKindFromExtra(null), null);
assert.equal(providerKindFromExtra({}), null);
assert.equal(providerKindFromExtra({ kind: " Bookmark " }), "bookmark");
assert.equal(providerKindFromExtra({ url: "https://example.com" }), "bookmark");
assert.equal(providerKindFromExtra({ virtual: true }), "virtual");
assert.equal(providerKindFromExtra({ virtual: false }), null);
assert.equal(providerKindFromExtra({ kind: "folder", virtual: true }), "folder");

const providerListing = providerListingFromDir({
  uri: "awam://browser/bookmarks",
  backend: "browser",
  entries: [
    {
      name: "GitHub",
      uri: "awam://browser/bookmarks/GitHub",
      isDir: false,
      isFile: true,
      size: 0,
      extra: { kind: "bookmark", url: "https://github.com" },
    },
    {
      name: "Collections",
      uri: "awam://browser/bookmarks/Collections",
      isDir: true,
      isFile: false,
      size: 0,
      extra: { virtual: true },
    },
    {
      name: "plain.txt",
      uri: "awam://browser/bookmarks/plain.txt",
      isDir: false,
      isFile: true,
      size: 1,
    },
  ],
});
assert.deepEqual(
  providerListing.entries.map((entry) => entry.providerKind),
  ["bookmark", "virtual", null],
);
assert.equal(
  specialEntryIcon(providerListing.entries[0]!, "awam://browser/bookmarks"),
  "bookmark",
);
assert.equal(
  specialEntryIcon(providerListing.entries[1]!, "awam://browser/bookmarks"),
  "provider",
);

const roots = providerRootEntriesFromBackends([
  {
    id: "knowledge",
    name: "Knowledge",
    description: "notes",
    healthy: true,
    capabilities: ["list"],
  },
]);
assert.equal(roots[0]?.providerKind, "backend");
assert.equal(specialEntryIcon(roots[0]!, "awam://root"), "provider");

console.log("entry-icon-checks: ok");
