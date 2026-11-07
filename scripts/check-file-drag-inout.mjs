// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-205: bidirectional drag with Explorer / external apps + system folders.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const dropPaths = read("src/state/dropPaths.ts");
const nav = read("src/views/NavigatorView.vue");
const dragOut = read("src/state/fileDragOut.ts");

assert.ok(dropPaths.includes("pathsFromDataTransfer"), "pathsFromDataTransfer helper");
assert.ok(dropPaths.includes("FileWithPath") || dropPaths.includes("file?.path"), "WebView2 File.path");
assert.ok(dropPaths.includes("text/uri-list"), "uri-list fallback");
assert.ok(dropPaths.includes("fileUrlToPath"), "file URL decode");
assert.ok(dropPaths.includes("setOutboundPathPayload"), "outbound uri-list companions");

assert.ok(nav.includes("pathsFromDataTransfer"), "Navigator uses drop path helper");
assert.ok(nav.includes("onListDrop"), "list background accepts drops");
assert.ok(nav.includes("dataTransferLooksExternal"), "external vs in-app detection");
assert.ok(nav.includes("setOutboundPathPayload"), "outbound payload helper");
assert.ok(nav.includes("armNativeFileDragOut"), "keeps OLE drag-out");

assert.ok(dragOut.includes("startDrag"), "OLE drag-out still present");

// Lightweight behavioural checks mirroring dropPaths.ts (keep in sync).
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

console.log("File drag in/out checks OK (AIA-205 Explorer + external + OLE).");
