// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { isRecyclePath, RECYCLE_PATH } from "../src/state/recyclePath.ts";
import { recycleListingFromItems } from "../src/state/recycleListing.ts";

assert.equal(RECYCLE_PATH, "afm://recycle-bin");
assert.equal(isRecyclePath("afm://recycle-bin"), true);
assert.equal(isRecyclePath("afm://recycle-bin/"), true);
assert.equal(isRecyclePath("afm://guide"), false);

const listing = recycleListingFromItems([
  {
    name: "a.txt",
    originalPath: "C:\\Temp\\a.txt",
    recyclePath: "::{trash}\\a",
  },
]);
assert.equal(listing.path, RECYCLE_PATH);
assert.equal(listing.fileCount, 1);
assert.equal(listing.entries[0]?.path, "C:\\Temp\\a.txt");

console.log("recycle-bin checks OK (AIA-193).");
