// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const tree = readFileSync(join(root, "src/components/shell/FolderTree.vue"), "utf8");
const branch = readFileSync(join(root, "src/components/shell/FolderTreeBranch.vue"), "utf8");
const helpers = readFileSync(join(root, "src/state/folderTree.ts"), "utf8");

const sidenav = shell.indexOf('class="sidenav');
const treeUse = shell.indexOf("<FolderTree");
const navEnd = shell.indexOf("</nav>", sidenav);
if (!(sidenav >= 0 && treeUse > sidenav && treeUse < navEnd)) {
  console.error("Folder tree must stay inside the product sidenav");
  process.exit(1);
}
if (
  !shell.includes('v-show="isNavigatorRoute"') &&
  !shell.includes('v-if="isNavigatorRoute"')
) {
  console.error("folder tree must be navigator-only (v-show/v-if)");
  process.exit(1);
}
if (!shell.includes("SIDE_NAV_ITEMS")) {
  console.error("product sidenav items must remain");
  process.exit(1);
}

// AIA-236: Explorer-like This PC + drive roots; no discrete entry sections.
for (const needle of [
  "guide.rootLabel",
  "GUIDE_PATH",
  "FolderTreeBranch",
  "drivesList",
  "loadDrives",
  'data-tree-root="this-pc"',
  "thisPcExpanded",
  "isOsHierarchyPath",
]) {
  if (!tree.includes(needle)) {
    console.error(`FolderTree missing ${needle}`);
    process.exit(1);
  }
}

for (const banned of [
  "guide.sections.folders",
  "folder-tree-section",
  "folder-tree-heading--toggle",
  "toggleFolderTreeSection",
  "loadFolderTreeSectionCollapse",
  "folderTreeSections",
  "shellPlaces",
  "userFolderItems",
  "RECYCLE_PATH",
  'highlight-mode="entry"',
  'data-section="folders"',
  'data-section="onedrive"',
  'data-section="libraries"',
  'data-section="network"',
  'data-section="recycle"',
]) {
  if (tree.includes(banned)) {
    console.error(`AIA-236: FolderTree must not keep discrete entry block ${banned}`);
    process.exit(1);
  }
}

if (!branch.includes("tree-branch--nested") || !branch.includes("tree-children")) {
  console.error("nested hierarchy visual markers required");
  process.exit(1);
}
for (const needle of [
  "navigator_list",
  "directoryChildren",
  "navigateTo",
  "treeRowHighlight",
  "treeShouldAutoExpand",
  "userCollapsed",
]) {
  if (!branch.includes(needle)) {
    console.error(`FolderTreeBranch missing ${needle}`);
    process.exit(1);
  }
}
if (!branch.includes("tree-label--current") || !branch.includes("tree-label--selected")) {
  console.error("need selected vs current highlight classes");
  process.exit(1);
}
if (!helpers.includes("treeShouldAutoExpand") || !helpers.includes("isOsHierarchyPath")) {
  console.error("folderTree helpers missing AIA-236 auto-expand APIs");
  process.exit(1);
}

try {
  readFileSync(join(root, "src/state/folderTreeSections.ts"));
  console.error("AIA-236: folderTreeSections.ts must be removed with entry sections");
  process.exit(1);
} catch {
  /* expected missing */
}

const registerUrl = pathToFileURL(
  join(root, "scripts/register-ts-resolve.mjs"),
).href;
const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/folder-tree-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);

console.log("folder-tree structural checks OK (AIA-236).");
