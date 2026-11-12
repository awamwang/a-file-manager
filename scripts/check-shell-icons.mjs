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
assert.ok(
  !rust.includes("SHELL_ICON_PARALLELISM") && !rust.includes("resolve_icons_parallel"),
  "must not parallelize IShellItemImageFactory (breaks SageThumbs / shell handlers)",
);
assert.ok(rust.includes("type_icon_cache_key"), "same-extension shell icon reuse (icon-only)");
assert.ok(
  rust.includes("system_thumbnail_data_url")
    && /system_thumbnail_data_url[\s\S]*?\.or_else/.test(rust),
  "thumbnail path must try Shell handlers then fall back to shell icon",
);
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
assert.ok(
  view.includes("preferThumbnail") && view.includes("entryWantsContentThumbnail"),
  "icon view: only media types prefer content thumbnails",
);
const thumbPolicy = readFileSync(
  join(root, "src/state/shellIconThumbPolicy.ts"),
  "utf8",
);
assert.ok(thumbPolicy.includes("entryWantsContentThumbnail"));
assert.ok(thumbPolicy.includes("mp4") && thumbPolicy.includes("png"));
assert.ok(state.includes("promotePriorityWindow"), "stable top-to-bottom priority window");
assert.ok(state.includes("takeNextBatch"), "folder/icon-first batch drain");
assert.ok(
  state.includes("suspendQueuedContentThumbnails"),
  "scroll fling can drop queued content thumbs",
);
assert.ok(
  view.includes("allowContentThumbnails") || view.includes("contentThumbnailsAllowed"),
  "viewport/scroll gate for content thumbnails",
);
assert.ok(
  view.includes("computeThumbRequestWindow") && view.includes("syncThumbRequestsForPane"),
  "FIL-50: viewport geometry thumb window (full screen + row overscan)",
);
assert.ok(
  view.includes("scheduleViewportIconObservers") || view.includes("refreshViewportIconRequests"),
  "viewport thumb sync scheduler",
);
assert.ok(
  !/new\s+IntersectionObserver/.test(view),
  "FIL-50: must not rebind IntersectionObserver for thumbs (invalidates slow batches)",
);
assert.ok(
  !/requestIconsFor\(visibleEntries\.value\.map/.test(view),
  "must not request icons for the entire filtered listing",
);
assert.ok(shellImg.includes("decoding=\"async\"") || shellImg.includes("decoding='async'"));
assert.ok(shellImg.includes("iconUrlFor"));
assert.ok(shellImg.includes("shell-icon-slot"), "soft near-transparent placeholder before final icon");
assert.ok(!shellImg.includes("FluentIcon"), "must not show Fluent glyph then swap to shell/thumb");

console.log("Shell icons checks OK (AIA-65/182/143/278 system thumbnails + scroll queue).");
