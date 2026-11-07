// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const tree = readFileSync(join(root, "src/components/shell/FolderTree.vue"), "utf8");
const branch = readFileSync(join(root, "src/components/shell/FolderTreeBranch.vue"), "utf8");

const sidenav = shell.indexOf('class="sidenav');
const treeUse = shell.indexOf("<FolderTree");
const navEnd = shell.indexOf("</nav>", sidenav);
if (!(sidenav >= 0 && treeUse > sidenav && treeUse < navEnd)) {
  console.error("Folder tree must stay inside the product sidenav");
  process.exit(1);
}
if (!shell.includes('v-if="isNavigatorRoute"')) {
  console.error("folder tree must be navigator-only");
  process.exit(1);
}
if (!shell.includes("SIDE_NAV_ITEMS")) {
  console.error("product sidenav items must remain");
  process.exit(1);
}
for (const needle of ["guide.sections.folders", "guide.title", "GUIDE_PATH", "FolderTreeBranch"]) {
  if (!tree.includes(needle)) {
    console.error(`FolderTree missing ${needle}`);
    process.exit(1);
  }
}
for (const needle of ["navigator_list", "directoryChildren", "navigateTo", "treeRowHighlight"]) {
  if (!branch.includes(needle)) {
    console.error(`FolderTreeBranch missing ${needle}`);
    process.exit(1);
  }
}
if (!tree.includes('highlight-mode="entry"')) {
  console.error("AIA-230: 常用文件夹 roots must use entry highlight (no sticky selected)");
  process.exit(1);
}
if (branch.includes("tree-label--active") && !branch.includes("tree-label--current")) {
  console.error("AIA-230: entry current highlight must replace sticky --active selected");
  process.exit(1);
}
if (!branch.includes("tree-label--current") || !branch.includes("tree-label--selected")) {
  console.error("AIA-230: need selected vs current highlight classes");
  process.exit(1);
}

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/folder-tree-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
