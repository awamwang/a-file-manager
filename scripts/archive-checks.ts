// SPDX-License-Identifier: GPL-3.0-only
// Behavioral unit for the ZIP context-menu gating. Pinned by scripts/check-archive.mjs.
// Incremental over context-menu-model-checks.ts (which only covers a directory): here we
// lock the zip-file scenarios — extractZip is enabled only for .zip files, case-insensitively.
import assert from "node:assert/strict";
import { buildContextMenuItems } from "../src/components/navigator/contextMenuModel.ts";

type Kind = "file" | "dir";
const mk = (name: string, kind: Kind) => ({
  name,
  path: `C:/afm-zip-fixture/${name}`,
  kind,
  size: kind === "file" ? 8 : 0,
});

const dir = mk("Docs", "dir");
const zipLower = mk("bundle.zip", "file");
const zipUpper = mk("BUNDLE.ZIP", "file");
const txt = mk("note.txt", "file");

const enabled = (entry: ReturnType<typeof mk>, id: string) =>
  buildContextMenuItems(entry, { canPaste: true }).find((item) => item.id === id)?.enabled;

// Compress is always offered (single-entry v1 baseline).
for (const entry of [dir, zipLower, zipUpper, txt]) {
  assert.equal(enabled(entry, "compressZip"), true, `compressZip must be enabled for ${entry.name}`);
}

// Extract is gated by isZipEntry: only files whose name ends with .zip, case-insensitive.
assert.equal(enabled(dir, "extractZip"), false, "a directory must not offer extract");
assert.equal(enabled(txt, "extractZip"), false, "a non-zip file must not offer extract");
assert.equal(enabled(zipLower, "extractZip"), true, ".zip file must offer extract");
assert.equal(enabled(zipUpper, "extractZip"), true, ".ZIP (uppercase) must still offer extract");

console.log("archive-checks OK (compress always on; extractZip gated by case-insensitive .zip).");
