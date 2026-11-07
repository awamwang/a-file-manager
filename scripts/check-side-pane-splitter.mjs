// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-256: draggable Details / Preview side pane width (persisted).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const state = read("src/state/sidePaneWidth.ts");
const preview = read("src/components/navigator/PreviewPane.vue");
const details = read("src/components/navigator/DetailsPane.vue");
const en = JSON.parse(read("src/i18n/locales/en.json"));

assert.ok(state.includes("SIDE_PANE_MIN_WIDTH"));
assert.ok(state.includes("afm.previewPaneWidth"));
assert.ok(state.includes("afm.detailsPaneWidth"));
assert.ok(state.includes("clampSidePaneWidth"));
assert.ok(state.includes("setPreviewPaneWidth"));
assert.ok(state.includes("setDetailsPaneWidth"));
assert.ok(state.includes("applyPreviewPaneWidth"), "live drag must not thrash storage");
assert.ok(state.includes("applyDetailsPaneWidth"));

for (const [name, src] of [
  ["PreviewPane", preview],
  ["DetailsPane", details],
]) {
  assert.ok(src.includes('class="side-pane-splitter"'), `${name}: splitter`);
  assert.ok(src.includes("onResizeStart"), `${name}: pointer drag start`);
  assert.ok(src.includes("onResizeMove"), `${name}: pointer drag move`);
  assert.ok(src.includes("onSplitterKey"), `${name}: keyboard resize`);
  assert.ok(src.includes('role="separator"'), `${name}: a11y separator`);
  assert.ok(src.includes("min-width: 15rem"), `${name}: CSS floor 15rem`);
  assert.ok(src.includes(":style=\"{ width:"), `${name}: inline width binding`);
}

assert.ok(preview.includes("loadPreviewPaneWidth"));
assert.ok(preview.includes("applyPreviewPaneWidth"));
assert.ok(preview.includes("setPreviewPaneWidth"));
assert.ok(preview.includes("preview.resize"));
assert.ok(details.includes("loadDetailsPaneWidth"));
assert.ok(details.includes("applyDetailsPaneWidth"));
assert.ok(details.includes("setDetailsPaneWidth"));
assert.ok(details.includes("detailsPane.resize"));

assert.equal(en.preview.resize, "Resize preview pane");
assert.equal(en.detailsPane.resize, "Resize details pane");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/side-pane-width-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);

console.log("side-pane-splitter checks OK (AIA-256).");
