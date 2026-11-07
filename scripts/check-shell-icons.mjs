// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const rust = readFileSync(join(root, "src-tauri/src/shell_icons/mod.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const state = readFileSync(join(root, "src/state/shellIcons.ts"), "utf8");

assert.ok(cargo.includes("file_icon_provider"));
assert.ok(lib.includes("shell_icons::shell_icon_get"));
assert.ok(rust.includes("get_file_icon"));
assert.ok(
  rust.includes("IShellItemImageFactory") && rust.includes("SIIGBF_THUMBNAILONLY"),
  "AIA-143 must reuse Shell IShellItemImageFactory thumbnails",
);
assert.ok(rust.includes("SIIGBF_INCACHEONLY"), "prefer Explorer thumbnail cache first");
assert.ok(
  !rust.includes("image_thumbnail_data_url") && !rust.includes("load_from_memory"),
  "must not self-decode images into thumbnails",
);
assert.ok(state.includes("shell_icon_get_many"));
assert.ok(view.includes("requestShellIcons"));
assert.ok(view.includes("shell-icon"));
assert.ok(view.includes("IntersectionObserver"), "viewport-only icon/thumbnail load (AIA-182)");
assert.ok(view.includes("bindIconObserver") || view.includes("refreshViewportIconObservers"));
assert.ok(
  !/requestIconsFor\(visibleEntries\.value\.map/.test(view),
  "must not request icons for the entire filtered listing",
);

console.log("Shell icons checks OK (AIA-65/182/143 system thumbnails).");
