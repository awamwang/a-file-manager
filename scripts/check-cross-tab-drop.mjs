// SPDX-License-Identifier: GPL-3.0-or-later
/** Cross-tab file drop: 500ms tab dwell, same-volume cut / cross-volume copy, release hint. */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const hover = read("src/state/dnd/hover.ts");
const session = read("src/state/dnd/session.ts");
const surface = read("src/state/dnd/surface.ts");
const location = read("src/state/dnd/location.ts");
const dragOut = read("src/state/dnd/dragOut.ts");
const mode = read("src/state/dnd/mode.ts");
const index = read("src/state/dnd/index.ts");
const shell = read("src/components/shell/AppShell.vue");
const nav = read("src/views/NavigatorView.vue");
const tree = read("src/components/shell/FolderTreeBranch.vue");
const overlay = read("src/components/shell/DropHintOverlay.vue");
const conf = JSON.parse(read("src-tauri/tauri.conf.json"));

// 没有 DOM dragover/dragenter/drop，标签上的 500ms dwell 与提示浮层永远不会被驱动。
assert.equal(
  conf.app.windows.find((w) => w.label === "main")?.dragDropEnabled,
  false,
  "native WebView2 drag target must not be replaced by Tauri's file-drop handler",
);

// 标签悬停 500ms，与目录树 / Places 保持一致。
assert.ok(hover.includes("DROP_HOVER_MS = 500"), "tree dwell stays 500ms");
assert.ok(hover.includes("TAB_DROP_HOVER_MS = 500"), "tab dwell is 500ms");
assert.ok(
  shell.includes("createDropHoverScheduler(TAB_DROP_HOVER_MS)"),
  "tab hover scheduler uses the 500ms dwell",
);
assert.ok(shell.includes("createDropHoverScheduler()"), "places hover keeps default dwell");
assert.ok(tree.includes("createDropHoverScheduler()"), "tree hover keeps default dwell");
assert.ok(shell.includes("switchNavigatorTab(groupIndex)"), "dwell activates the hovered tab");
assert.ok(index.includes("TAB_DROP_HOVER_MS"), "500ms dwell exported from dnd barrel");

// 同盘剪切 / 跨盘复制规则保持单一来源。
assert.ok(mode.includes('sameVolume ? "move" : "copy"'), "volume rule");
assert.ok(mode.includes("volumeKey"), "drive/UNC volume key");

// 拖回原目录 = 无效操作：不提示、不高亮、不切标签、不发起任何文件操作。
assert.ok(location.includes("isSourceInDest"), "same-location detector");
assert.ok(location.includes("effectiveDropSources"), "in-place sources are filtered");
assert.ok(surface.includes("effectiveDropSources"), "shared surface uses the location guard");
assert.ok(surface.includes("resolveFolderDrop"), "dragover and drop share one resolution");
assert.ok(surface.includes('mode !== "move"'), "explicit copy/link stays allowed in place");
assert.ok(surface.includes('noop: "same-location"'), "same-location drop is a silent no-op");
assert.ok(surface.includes('dropEffect = "none"'), "blocked target refuses the drop");
assert.ok(
  surface.includes("hideDropHint(hintKey)") && surface.includes("event.stopPropagation()"),
  "blocked target clears its hint without bubbling",
);
assert.ok(nav.includes("if (blocked)"), "list background honours the block");
assert.ok(
  shell.includes("if (mode == null)") && shell.includes("clearTabFileDropHover()"),
  "tab hover switch is cancelled for a blocked target",
);
assert.ok(index.includes("resolveFolderDrop"), "resolution exported from dnd barrel");

// 拖动会话：切标签后仍能拿到源路径，保证提示与实际动作一致。
for (const token of [
  "beginDragSources",
  "activeDragSources",
  "endDragSession",
  "showDropHint",
  "hideDropHint",
  "dropHint",
]) {
  assert.ok(session.includes(token), `session exports ${token}`);
  assert.ok(index.includes(token), `dnd barrel re-exports ${token}`);
}
assert.ok(surface.includes("activeDragSources()"), "dragover preview uses session sources");
assert.ok(surface.includes("showDropHint(event"), "folder dragover shows the release hint");
assert.ok(surface.includes("endDragSession()"), "drop ends the session / clears the hint");
assert.ok(dragOut.includes("beginDragSources(cleaned)"), "press registers drag sources");
assert.ok(dragOut.includes("endDragSession()"), "plain click clears drag sources");
assert.ok(
  shell.includes("onGlobalDragEnd") && shell.includes('"dragend", onGlobalDragEnd'),
  "cancelled drags clean up",
);

// 标签 / 列表 / 树 / 收藏夹都参与提示，且各自 dragleave 只清自己的提示。
assert.ok(shell.includes("hintKey: `tab:${groupIndex}`"), "tab hint key");
assert.ok(shell.includes("hideDropHint(`tab:${groupIndex}`)"), "tab dragleave clears hint");
assert.ok(shell.includes("hideDropHint(path)"), "places dragleave clears hint");
assert.ok(nav.includes("showDropHint(event, mode, label, destPath)"), "list dragover hint");
assert.ok(nav.includes("hideDropHint(destPath)"), "list dragleave clears hint");
assert.ok(tree.includes("hideDropHint(props.path)"), "tree dragleave clears hint");
assert.ok(shell.includes("<DropHintOverlay />"), "hint overlay mounted");
assert.ok(
  shell.includes('@dragover="onTabFileDragOver') && shell.includes('@drop="onTabFileDrop'),
  "tab accepts in-app file drop",
);

// 浮层：跟随光标、不吃 drop 事件、按模式给文案。
assert.ok(overlay.includes('Teleport to="body"'), "overlay teleports to body");
assert.ok(overlay.includes("pointer-events: none"), "overlay never blocks the drop");
assert.ok(
  overlay.includes("drop-hint--") && overlay.includes("hint.mode"),
  "overlay styles by transfer mode",
);
for (const key of ["dropHintMove", "dropHintCopy", "dropHintLink"]) {
  assert.ok(overlay.includes(key), `overlay uses ${key}`);
}
assert.ok(overlay.includes("position: fixed"), "fixed cursor-following position");

const localeIds = ["en", "zh-CN", "ja", "de", "fr"];
for (const id of localeIds) {
  const locale = JSON.parse(read(`src/i18n/locales/${id}.json`));
  assert.ok(locale.navigator.dropHintMove, `${id} cut hint`);
  assert.ok(locale.navigator.dropHintCopy, `${id} copy hint`);
  assert.ok(locale.navigator.dropHintLink, `${id} link hint`);
}

const runtime = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/cross-tab-drop-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (runtime.stdout) process.stdout.write(runtime.stdout);
if (runtime.stderr) process.stderr.write(runtime.stderr);
if (runtime.status !== 0) process.exit(runtime.status ?? 1);

console.log("Cross-tab drop checks OK (500ms dwell + volume mode + release hint + same-location guard).");
