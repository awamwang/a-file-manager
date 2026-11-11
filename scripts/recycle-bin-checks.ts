// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { isRecyclePath, RECYCLE_PATH } from "../src/state/recycle/path.ts";
import { recycleListingFromItems } from "../src/state/recycle/listing.ts";
import { RECYCLE_SHELL_TARGET } from "../src/state/recycle/openSystem.ts";

assert.equal(RECYCLE_PATH, "afm://recycle-bin");
assert.equal(isRecyclePath("afm://recycle-bin"), true);
assert.equal(isRecyclePath("afm://recycle-bin/"), true);
assert.equal(isRecyclePath("afm://guide"), false);
assert.equal(RECYCLE_SHELL_TARGET, "shell:RecycleBinFolder");

const listing = recycleListingFromItems([
  {
    name: "a.txt",
    originalPath: "C:\\Temp\\afm-fixture\\a.txt",
    recyclePath: "::{trash}\\a",
  },
]);
assert.equal(listing.path, RECYCLE_PATH);
assert.equal(listing.fileCount, 1);
assert.equal(listing.entries[0]?.path, "C:\\Temp\\afm-fixture\\a.txt");

console.log("recycle-bin checks OK (AIA-193).");
