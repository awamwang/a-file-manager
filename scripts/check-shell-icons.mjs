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
const shellImg = readFileSync(
  join(root, "src/components/navigator/ShellIconImg.vue"),
  "utf8",
);

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
assert.ok(rust.includes("spawn_blocking"), "AIA-278: Shell work off async runtime");
assert.ok(rust.includes("SHELL_ICON_PARALLELISM"), "Phase B: bounded batch parallelism");
assert.ok(rust.includes("resolve_icons_parallel"), "Phase B: parallel batch resolver");
assert.ok(rust.includes("thread::scope"), "Phase B: scoped workers for COM STA");
assert.ok(rust.includes("type_icon_cache_key"), "Phase B: same-extension shell icon reuse");
assert.ok(state.includes("shell_icon_get_many"));
assert.ok(state.includes("clearShellIconQueue"), "cancel queue on folder change");
assert.ok(state.includes("cancelShellIconRequests"), "cancel out-of-view thumbs");
assert.ok(state.includes("BATCH_THUMB"), "small thumbnail batches");
assert.ok(state.includes("priority"), "viewport priority enqueue");
assert.ok(state.includes("SHELL_ICON_MAX_ENTRIES"), "AIA-279 entry budget");
assert.ok(state.includes("SHELL_ICON_MAX_BYTES"), "AIA-279 byte budget");
assert.ok(state.includes("trimToBudget"), "AIA-279 LRU trim");
assert.ok(rust.includes("BoundedIconCache") || rust.includes("MAX_ICON_CACHE_ENTRIES"));
assert.ok(view.includes("requestShellIcons"));
assert.ok(view.includes("ShellIconImg"), "per-row icon isolate");
assert.ok(view.includes("clearShellIconQueue"));
assert.ok(view.includes("cancelShellIconRequests"));
assert.ok(view.includes("IntersectionObserver"), "viewport-only icon/thumbnail load (AIA-182)");
assert.ok(view.includes("bindIconObserver") || view.includes("refreshViewportIconObservers"));
assert.ok(
  !/requestIconsFor\(visibleEntries\.value\.map/.test(view),
  "must not request icons for the entire filtered listing",
);
assert.ok(shellImg.includes("decoding=\"async\"") || shellImg.includes("decoding='async'"));
assert.ok(shellImg.includes("iconUrlFor"));
assert.ok(shellImg.includes("shell-icon-slot"), "soft near-transparent placeholder before final icon");
assert.ok(!shellImg.includes("FluentIcon"), "must not show Fluent glyph then swap to shell/thumb");

console.log("Shell icons checks OK (AIA-65/182/143/278 system thumbnails + scroll queue).");
