// SPDX-License-Identifier: GPL-3.0-or-later
// Virtual/provider entries (SFTP / rclone remotes, bookmarks, guide and
// recycle-bin pseudo paths) have no locally readable file. The preview,
// properties and details panes must show listing-derived info instead of
// piping `awam://…` into preview_load / navigator_stat and surfacing "Not a
// file" / "Path does not exist" errors.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const module = read("src/state/virtualEntryInfo.ts");
const preview = read("src/components/navigator/PreviewPane.vue");
const props = read("src/components/navigator/PropertiesPanel.vue");
const details = read("src/components/navigator/DetailsPane.vue");
const nav = read("src/views/NavigatorView.vue");
const glyphs = read("src/components/icons/glyphs.ts");
const copyComponent = read("src/components/common/CopyableFieldValue.vue");

// Shared module: recognizes provider / guide / recycle paths and builds rows.
assert.ok(module.includes("export function isVirtualEntryPath"));
assert.ok(
  module.includes("isProviderPath(path) || isGuidePath(path) || isRecyclePath(path)"),
  "must recognize all three virtual path families",
);
assert.ok(module.includes("export function virtualEntryInfo"));
assert.ok(module.includes("isGuidePath(raw)"));
assert.ok(module.includes("isRecyclePath(raw)"));
assert.ok(module.includes("isProviderPath(raw)"));
assert.ok(module.includes("providerBackendId(raw)"));
assert.ok(module.includes("getProviderQuery(raw)"));
assert.ok(module.includes("providerLeafName(path)"));
assert.ok(module.includes("formatBytes(entry.size)"));
assert.ok(module.includes("MAX_QUERY_ROWS = 8"));
assert.ok(module.includes("copyable?: boolean"), "VirtualInfoRow must carry a copyable flag");
assert.ok(
  module.includes('key.toLowerCase() === "magnet"'),
  "only magnet query rows should be click-to-copy",
);
for (const kind of [
  "kindGuide",
  "kindRecycle",
  "kindProviderRoot",
  "kindBackend",
  "kindBookmark",
  "kindRemoteFolder",
  "kindRemoteFile",
  "kindRemoteItem",
  "hintSpecial",
  "hintRemote",
]) {
  assert.ok(module.includes(`virtualInfo.${kind}`), `virtualInfo.${kind}`);
}

// Glyphs referenced by the info cards must exist in the icon registry.
for (const glyph of [
  "desktop",
  "recycle",
  "provider",
  "cloud",
  "bookmark",
  "folder",
  "document",
]) {
  assert.match(glyphs, new RegExp(`^  ${glyph}:`, "m"), `glyph ${glyph}`);
}

// Preview: virtual branch runs BEFORE the local preview_load IPC call.
assert.ok(preview.includes("import {\n  isVirtualEntryPath,"));
assert.ok(preview.includes("virtualInfo.value = virtualEntryInfo(path, props.entry"));
assert.ok(
  preview.indexOf("isVirtualEntryPath(path)") <
    preview.indexOf('invoke<PreviewPayload>("preview_load"'),
  "virtual paths must skip preview_load",
);
assert.ok(preview.includes('v-else-if="virtualInfo"'));
assert.ok(preview.includes("preview-info-hint"));
assert.ok(preview.includes("virtualInfo.rows"));

// Properties: virtual branch runs BEFORE navigator_stat; attribute controls
// are hidden for virtual entries (they cannot be toggled on a remote item).
assert.ok(props.includes("virtualInfo.value = virtualEntryInfo(props.path, props.entry"));
assert.ok(
  props.indexOf("isVirtualEntryPath(props.path)") <
    props.indexOf('invoke<EntryStat>("navigator_stat"'),
  "virtual paths must skip navigator_stat",
);
assert.ok(props.includes("const displayRows = computed"));
assert.ok(props.includes('v-else-if="stat || virtualInfo"'));
assert.ok(props.includes('v-if="stat" class="props-row props-row--attrs"'));
assert.ok(props.includes("props-hint"));

// Details: single virtual selection uses the same listing-derived info.
assert.ok(details.includes("virtualInfo.value = virtualEntryInfo(path, singleEntry.value"));
assert.ok(
  details.indexOf("isVirtualEntryPath(path)") <
    details.indexOf('invoke<EntryStat>("navigator_stat"'),
  "virtual paths must skip navigator_stat",
);
assert.ok(details.includes('v-else-if="stat || virtualInfo"'));
assert.ok(details.includes("displayRows"));

// magnet fields: all three panes render the shared click-to-copy control.
for (const [name, source] of [
  ["preview", preview],
  ["properties", props],
  ["details", details],
]) {
  assert.ok(source.includes('v-if="row.copyable"'), `${name}: gate on row.copyable`);
  assert.ok(source.includes("CopyableFieldValue"), `${name}: use shared copy control`);
  assert.ok(
    source.includes(':label="t(\'virtualInfo.copyMagnet\')"'),
    `${name}: localized copy tooltip`,
  );
}
assert.ok(copyComponent.includes("navigator.clipboard.writeText(props.value)"));
assert.ok(copyComponent.includes(":name=\"copied ? 'check' : 'copy'\""), "inline copy feedback");
// WebView2 can reject the async clipboard API (unfocused document / permission
// policy); the control must fall back to the legacy selection copy and report
// failures, otherwise a click looks like a no-op.
assert.ok(
  copyComponent.includes('document.execCommand("copy")'),
  "fall back to document.execCommand when the clipboard API is denied",
);
assert.ok(copyComponent.includes('t("virtualInfo.copied")'), "success feedback");
assert.ok(copyComponent.includes('t("virtualInfo.copyFailed")'), "failure feedback");

// NavigatorView: Space preview on an all-virtual selection toggles the
// built-in pane (info card) instead of a raw "no local path" toast.
assert.ok(nav.includes("paths.every((path) => isVirtualEntryPath(path))"));
assert.ok(nav.includes("previewBridgeForcePane.value = !previewBridgeForcePane.value"));

// NavigatorView: all three panels receive the listing row(s) as info source.
assert.ok(nav.includes("function findVisibleEntry"));
assert.ok(nav.includes(':entry="previewEntry"'));
assert.ok(nav.includes(':entry="propertiesEntry"'));
assert.ok(nav.includes(':entries="visibleEntries"'));

// i18n: every locale carries the virtual-info strings.
const locales = ["en", "zh-CN", "ja", "de", "fr"];
const keys = [
  "kindGuide",
  "kindRecycle",
  "kindRemoteItem",
  "kindProviderRoot",
  "kindBookmark",
  "kindBackend",
  "kindRemoteFolder",
  "kindRemoteFile",
  "recycleName",
  "remoteServicesName",
  "backend",
  "hintSpecial",
  "hintRemote",
  "copyMagnet",
  "copied",
  "copyFailed",
];
for (const locale of locales) {
  const data = JSON.parse(read(`src/i18n/locales/${locale}.json`));
  assert.ok(data.virtualInfo, `${locale}: virtualInfo section`);
  for (const key of keys) {
    assert.equal(
      typeof data.virtualInfo[key],
      "string",
      `${locale}: virtualInfo.${key} must be a string`,
    );
    assert.ok(data.virtualInfo[key].trim().length > 0, `${locale}: virtualInfo.${key} empty`);
  }
}

console.log("Virtual entry info checks OK (preview / properties / details × 5 locales).");
