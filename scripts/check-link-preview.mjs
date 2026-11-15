// SPDX-License-Identifier: GPL-3.0-or-later
// Link preview: `.lnk` shortcuts and broken reparse points preview as link
// info (target, 原目录) instead of "file not found" / opaque binary.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const previewRs = read("src-tauri/src/preview/mod.rs");
const shortcutRs = read("src-tauri/src/file_ops/shortcut.rs");
const pane = read("src/components/navigator/PreviewPane.vue");
const entryType = read("src/state/entryType.ts");
const nav = read("src/views/NavigatorView.vue");
const grouping = read("src/state/navigatorGrouping.ts");

// Rust: link payload + branch order (broken reparse first, then .lnk).
assert.ok(previewRs.includes("pub struct PreviewLinkInfo"));
assert.ok(previewRs.includes("pub link: Option<PreviewLinkInfo>"));
assert.ok(previewRs.includes("broken_reparse_link_info"));
assert.ok(previewRs.includes("clean_reparse_target"));
assert.ok(previewRs.includes('describe_link_target("symlink"'));
assert.ok(previewRs.includes('describe_link_target("shortcut"'));
assert.ok(previewRs.includes('if ext == "lnk"'));
assert.ok(
  previewRs.indexOf("broken_reparse_link_info(pb)") <
    previewRs.indexOf('if ext == "lnk"'),
  "broken reparse check must run before the .lnk branch",
);
assert.ok(
  previewRs.indexOf("broken_reparse_link_info(pb)") <
    previewRs.indexOf("!pb.is_file()"),
  "broken links must not fall into the 'Not a file' error",
);

// Rust: .lnk reader via COM (mirror of create_one_shortcut).
assert.ok(shortcutRs.includes("pub fn read_shortcut_target"));
assert.ok(shortcutRs.includes("STGM_READ"));
assert.ok(shortcutRs.includes("GetPath"));
assert.ok(shortcutRs.includes("IPersistFile"));

// Rust: tests cover lnk roundtrip + broken junction.
assert.ok(previewRs.includes("lnk_preview_returns_link_info"));
assert.ok(
  previewRs.includes("broken_junction_preview_returns_link_info_not_error"),
);

// FE: preview pane renders a link card, not the unsupported banner.
assert.ok(pane.includes("export type PreviewLinkInfo"));
assert.ok(pane.includes("link: PreviewLinkInfo | null"));
assert.ok(pane.includes("payload.kind === 'link' && payload.link"));
assert.ok(pane.includes("linkKindLabel"));
assert.ok(pane.includes("targetKindLabel"));
assert.ok(pane.includes("copyLinkTarget"));
assert.ok(pane.includes("preview.linkTargetMissing"));
assert.ok(pane.includes("preview.linkTargetUnresolved"));
assert.ok(pane.includes("preview.linkTargetDirLabel"));
assert.ok(pane.includes("preview.linkCopyTarget"));
assert.ok(pane.includes('name="link"'));

// FE: `.lnk` gets a dedicated display form (type label + grouping key).
assert.ok(entryType.includes('if (ext === "lnk") return "shortcut";'));
assert.ok(entryType.includes("shortcut: string;"));
assert.ok(nav.includes("navigator.typeShortcut"));
assert.ok(grouping.includes("shortcut: string;"));

// i18n: all locales carry the link-preview strings.
const locales = ["en", "zh-CN", "ja", "de", "fr"];
for (const locale of locales) {
  const data = JSON.parse(read(`src/i18n/locales/${locale}.json`));
  assert.ok(data.navigator.typeShortcut, `${locale}: navigator.typeShortcut`);
  for (const key of [
    "linkKindShortcut",
    "linkKindSymlink",
    "linkTargetLabel",
    "linkTargetUnresolved",
    "linkTargetMissing",
    "linkTargetKindLabel",
    "linkTargetKindDir",
    "linkTargetKindFile",
    "linkTargetDirLabel",
    "linkCopyTarget",
    "linkCopiedTarget",
  ]) {
    assert.ok(data.preview[key], `${locale}: preview.${key}`);
  }
}

console.log("Link preview checks OK (.lnk + broken reparse → target info card).");
