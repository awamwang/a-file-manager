// SPDX-License-Identifier: GPL-3.0-or-later
// Link preview: `.lnk` shortcuts and broken reparse points preview as link
// info (target, 原目录) instead of "file not found" / opaque binary. The link
// card keeps showing when only the target is gone (原目录 still exists), and
// existing dir targets / 原目录 open in a new navigator tab. The 属性 panel
// (navigator_stat) attaches the same link metadata.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const previewRs = read("src-tauri/src/preview/mod.rs");
const shortcutRs = read("src-tauri/src/file_ops/shortcut.rs");
const statRs = read("src-tauri/src/navigator/stat.rs");
const pane = read("src/components/navigator/PreviewPane.vue");
const props = read("src/components/navigator/PropertiesPanel.vue");
const entryType = read("src/state/entryType.ts");
const nav = read("src/views/NavigatorView.vue");
const grouping = read("src/state/navigatorGrouping.ts");

// Rust: link payload + branch order (broken reparse first, then .lnk).
assert.ok(previewRs.includes("pub struct PreviewLinkInfo"));
assert.ok(previewRs.includes("pub link: Option<PreviewLinkInfo>"));
assert.ok(previewRs.includes("pub target_dir_exists: bool"));
assert.ok(previewRs.includes("broken_reparse_link_info"));
assert.ok(previewRs.includes("clean_reparse_target"));
assert.ok(previewRs.includes("pub(crate) fn reparse_link_info"));
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

// Rust: navigator_stat (属性) attaches the same link metadata and never
// errors on broken reparse points.
assert.ok(statRs.includes("pub link: Option<PreviewLinkInfo>"));
assert.ok(statRs.includes("fn entry_link_info"));
assert.ok(statRs.includes("reparse_link_info(pb)"));
assert.ok(statRs.includes("read_shortcut_target"));
assert.ok(statRs.includes("fs::symlink_metadata(&pb)"), "broken link falls back to the link node");
assert.ok(statRs.includes("attr_is_dir"), "junction keeps its dir-ness when broken");
assert.ok(statRs.includes("is_dir && follows_target"), "no dir walk on broken links");
assert.ok(statRs.includes("broken_junction_stat_returns_link_info_not_error"));
assert.ok(statRs.includes("lnk_stat_attaches_shortcut_link_info"));

// FE: preview pane renders a link card, not the unsupported banner.
assert.ok(pane.includes("export type PreviewLinkInfo"));
assert.ok(pane.includes("targetDirExists: boolean"));
assert.ok(pane.includes("link: PreviewLinkInfo | null"));
assert.ok(pane.includes("payload.kind === 'link' && payload.link"));
assert.ok(pane.includes("linkKindLabel"));
assert.ok(pane.includes("targetKindLabel"));
assert.ok(pane.includes("copyLinkTarget"));
assert.ok(pane.includes("preview.linkTargetMissing"));
assert.ok(pane.includes("preview.linkTargetUnresolved"));
assert.ok(pane.includes("preview.linkTargetDirLabel"));
assert.ok(pane.includes("preview.linkCopyTarget"));
assert.ok(pane.includes("preview.linkCopiedTarget"));
assert.ok(pane.includes("preview.linkTargetMissingBadge"));
assert.ok(pane.includes('name="link"'));

// FE: missing target but surviving 原目录 → muted badge, no error banner;
// both gone → red banner. Clickable paths open a new navigator tab.
assert.ok(pane.includes("showMissingBanner"));
assert.ok(pane.includes("targetMissing"));
assert.ok(pane.includes("canOpenTargetInNewTab"));
assert.ok(pane.includes("openTargetInNewTab"));
assert.ok(pane.includes("openTargetDirInNewTab"));
assert.ok(pane.includes("newNavigatorTab"));
assert.ok(pane.includes("preview-link-open"));
assert.ok(pane.includes("preview-link-badge"));

// FE: 属性 panel shows link rows (链接类型 / 目标 / 原目录) with the same
// open-in-new-tab affordances.
assert.ok(props.includes("link?: PreviewLinkInfo | null"));
assert.ok(props.includes("linkKindText"));
assert.ok(props.includes("canOpenLinkTarget"));
assert.ok(props.includes("openLinkTargetInNewTab"));
assert.ok(props.includes("openLinkTargetDirInNewTab"));
assert.ok(props.includes("newNavigatorTab"));
assert.ok(props.includes("properties.linkKind"));
assert.ok(props.includes("preview.linkTargetMissingBadge"));
assert.ok(props.includes("props-link-open"));

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
  assert.ok(data.properties.linkKind, `${locale}: properties.linkKind`);
  for (const key of [
    "linkKindShortcut",
    "linkKindSymlink",
    "linkTargetLabel",
    "linkTargetUnresolved",
    "linkTargetMissing",
    "linkTargetMissingBadge",
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
