// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  FOLDER_TREE_SECTION_IDS,
  collapsedFolderTreeSections,
  isFolderTreeSectionExpanded,
  loadFolderTreeSectionCollapse,
  readCollapsedFolderTreeSections,
  setFolderTreeSectionExpanded,
  toggleFolderTreeSection,
  writeCollapsedFolderTreeSections,
} from "../src/state/folderTreeSections.ts";

const memory = new Map<string, string>();
const storage = {
  getItem(key: string) {
    return memory.has(key) ? memory.get(key)! : null;
  },
  setItem(key: string, value: string) {
    memory.set(key, value);
  },
  removeItem(key: string) {
    memory.delete(key);
  },
};
(globalThis as { localStorage?: typeof storage }).localStorage = storage;

memory.clear();
collapsedFolderTreeSections.value = new Set();
loadFolderTreeSectionCollapse();
assert.equal(isFolderTreeSectionExpanded("folders"), true);
assert.equal(isFolderTreeSectionExpanded("drives"), true);

toggleFolderTreeSection("folders");
assert.equal(isFolderTreeSectionExpanded("folders"), false);
assert.deepEqual([...readCollapsedFolderTreeSections()].sort(), ["folders"]);

setFolderTreeSectionExpanded("drives", false);
assert.deepEqual([...readCollapsedFolderTreeSections()].sort(), ["drives", "folders"]);

setFolderTreeSectionExpanded("folders", true);
assert.deepEqual([...readCollapsedFolderTreeSections()], ["drives"]);

writeCollapsedFolderTreeSections(["network", "bogus" as never, "libraries"]);
assert.deepEqual([...readCollapsedFolderTreeSections()].sort(), ["libraries", "network"]);

for (const id of FOLDER_TREE_SECTION_IDS) {
  assert.equal(typeof id, "string");
}

console.log("folder-tree section collapse checks OK.");
