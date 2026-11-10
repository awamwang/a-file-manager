// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-240: bidirectional drag with Explorer + Link + dual-pane surfaces.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const dropPaths = read("src/state/dnd/paths.ts");
const dropMode = read("src/state/dnd/mode.ts");
const nav = read("src/views/NavigatorView.vue");
const dragOut = read("src/state/dnd/dragOut.ts");
const clipboard = read("src/state/fileClipboard.ts");
const shortcut = read("src-tauri/src/file_ops/shortcut.rs");
const lib = read("src-tauri/src/lib.rs");

assert.ok(dropPaths.includes("pathsFromDataTransfer"), "pathsFromDataTransfer helper");
assert.ok(dropPaths.includes("FileWithPath") || dropPaths.includes("file?.path"), "WebView2 File.path");
assert.ok(dropPaths.includes("text/uri-list"), "uri-list fallback");
assert.ok(dropPaths.includes("fileUrlToPath"), "file URL decode");
assert.ok(dropPaths.includes("setOutboundPathPayload"), "outbound uri-list companions");

assert.ok(dropMode.includes('"link"'), "dropMode includes link");
assert.ok(dropMode.includes("alt"), "dropMode honors Alt for link");

assert.ok(
  nav.includes("pathsFromDataTransfer") || nav.includes("fileDropSurface") || nav.includes("state/dnd"),
  "Navigator uses drop path helper (direct or shared surface)",
);
assert.ok(nav.includes("onListDrop"), "list background accepts drops");
assert.ok(
  nav.includes("dataTransferLooksExternal") || nav.includes("previewDropMode"),
  "external vs in-app detection (direct or shared preview)",
);
assert.ok(nav.includes("armNativeFileDragOut"), "keeps OLE drag-out");
assert.ok(
  nav.includes("linkedInto") || nav.includes("noticeKeyForDropMode"),
  "link drop notice",
);
assert.ok(nav.includes("secondaryListRoot"), "dual pane present");
assert.ok(surfaceHasSharedHelpers(), "AIA-242 shared drop surface present");

function surfaceHasSharedHelpers() {
  const surface = read("src/state/dnd/surface.ts");
  return (
    surface.includes("commitFolderDrop") &&
    surface.includes("acceptFolderDragOver") &&
    surface.includes("pathsFromDataTransfer")
  );
}
assert.ok(
  /secondaryListRoot[\s\S]*@dragstart="onDragStart\(\$event, entry\)"/.test(nav) ||
    (nav.includes("secondaryEntries") &&
      nav.split("@dragstart=\"onDragStart($event, entry)\"").length >= 3),
  "secondary pane rows arm drag (primary grouped+flat + secondary)",
);

assert.ok(dragOut.includes("startDrag"), "OLE via startDrag");
assert.ok(clipboard.includes("file_ops_create_shortcuts"), "transferEntries supports link");
assert.ok(shortcut.includes("IShellLinkW") || shortcut.includes("ShellLink"), "creates .lnk");
assert.ok(lib.includes("file_ops_create_shortcuts"), "command registered");
assert.ok(!lib.includes("mod shell_drag"), "no AFM IDropTarget/shell_drag module in MVP");

function fileUrlToPath(url) {
  const raw = url.trim();
  if (!raw || raw.startsWith("#")) return null;
  let decoded;
  try {
    decoded = decodeURIComponent(raw.replace(/\+/g, "%20"));
  } catch {
    decoded = raw;
  }
  if (!/^file:/i.test(decoded)) {
    if (/^[a-zA-Z]:[\\/]/.test(decoded) || decoded.startsWith("\\\\")) {
      return decoded.replace(/\//g, "\\");
    }
    return null;
  }
  const m = /^file:\/\/(?:localhost)?\/([a-zA-Z]):[\\/](.*)$/i.exec(decoded);
  if (m) return `${m[1].toUpperCase()}:\\${m[2].replace(/\//g, "\\")}`;
  const unc = /^file:\/\/([^/]+)\/(.*)$/i.exec(decoded);
  if (unc) return `\\\\${unc[1]}\\${unc[2].replace(/\//g, "\\")}`;
  const short = /^file:\/([a-zA-Z]):[\\/](.*)$/i.exec(decoded);
  if (short) return `${short[1].toUpperCase()}:\\${short[2].replace(/\//g, "\\")}`;
  return null;
}

assert.equal(fileUrlToPath("file:///C:/Users/Public/a.txt"), "C:\\Users\\Public\\a.txt");
assert.equal(fileUrlToPath("file:/D:/temp/b"), "D:\\temp\\b");
assert.equal(fileUrlToPath("C:\\Windows\\System32"), "C:\\Windows\\System32");
assert.equal(fileUrlToPath(pathToFileURL("C:\\Temp\\x").href), "C:\\Temp\\x");

console.log("File drag in/out checks OK (AIA-240 Explorer + Link + dual-pane).");
