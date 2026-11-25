// SPDX-License-Identifier: GPL-3.0-or-later
// AFM-UX (F26 / F27 / F28 / F33) — accessibility SEMANTICS regression gate.
//
// This does NOT simulate a screen reader's audio (that needs manual NVDA/JAWS
// runs). It locks the ARIA *substrate* an assistive tech reads from the DOM, so
// the roles/states/landmarks delivered during the UX audit cannot silently
// regress:
//   • F26/F34 navigator list    -> role derived by view: listbox/option (icons/list)
//                                   OR grid/row/gridcell + columnheader+aria-sort (Details)
//                                   + aria-selected + aria-multiselectable
//   • F27  folder tree          -> role=tree + role=treeitem + role=group + aria-expanded
//   • F28  transient feedback    -> ToastHost live region (aria-live=polite) + DropHint status
//   • F33  chrome landmarks      -> AppShell <nav>/role=navigation + role=search + role=tablist/tab,
//                                   and the reverted mistakes must STAY reverted:
//                                   NO role=tabpanel and NO role=region wrapping the body/main.
//
// Static, deterministic, no app/CDP required. Run directly:
//   node scripts/check-a11y-semantics.mjs
// Register in package.json (owner's file) as:
//   "test:a11y-semantics": "node scripts/check-a11y-semantics.mjs"
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const list = read("src/views/NavigatorView.vue");
const tree = read("src/components/shell/FolderTree.vue");
const branch = read("src/components/shell/FolderTreeBranch.vue");
const shell = read("src/components/shell/AppShell.vue");
const toast = read("src/components/shell/ToastHost.vue");
const dropHint = read("src/components/shell/DropHintOverlay.vue");

// ---- F26/F34 — navigator list exposes a derived composite: listbox/option (non-Details)
//      or a proper grid (Details), both with selection + multiselect + accessible name ----
// Container role is view-derived, so both role values must be reachable from source:
assert.ok(
  list.includes('isDetailsView ? \'grid\' : \'listbox\''),
  "navigator container role must derive grid(Details) / listbox(other views)",
);
assert.ok(
  list.includes('isDetailsView ? \'row\' : \'option\''),
  "navigator row role must derive row(Details) / option(other views)",
);
// F34 — Details grid substrate must exist (this is what made the old header illegal in a listbox):
assert.ok(list.includes('role="gridcell"'), "Details rows must expose role=gridcell");
assert.ok(list.includes('role="columnheader"'), "Details column headers must expose role=columnheader");
assert.ok(list.includes(":aria-sort="), "sortable Details columns must expose aria-sort");
assert.ok(
  list.includes(":aria-multiselectable=\"true\"") || list.includes('aria-multiselectable="true"'),
  "multi-select list must set aria-multiselectable=true",
);
assert.ok(
  list.includes(":aria-selected=") || list.includes('aria-selected="'),
  "rows/cells must reflect selection via aria-selected",
);
assert.ok(
  list.includes(":aria-label=") || list.includes('aria-label="'),
  "list/grid must carry an accessible name (aria-label)",
);

// ---- F27 — folder tree is a proper tree/treeitem/group composite ----
assert.ok(tree.includes('role="tree"'), "folder tree root must expose role=tree");
assert.ok(tree.includes('role="treeitem"'), "folder tree nodes must expose role=treeitem");
assert.ok(tree.includes('role="group"'), "expanded tree children must be wrapped in role=group");
assert.ok(
  tree.includes(":aria-expanded=") || tree.includes('aria-expanded="'),
  "expandable tree items must expose aria-expanded",
);
assert.ok(
  branch.includes('role="treeitem"') && branch.includes('role="group"'),
  "recursive tree branch must keep treeitem + group roles",
);
assert.ok(
  branch.includes(":aria-expanded=") || branch.includes('aria-expanded="'),
  "recursive tree branch must keep aria-expanded",
);

// ---- F28 — transient feedback is announced via live regions ----
assert.ok(
  toast.includes('aria-live="polite"'),
  "ToastHost must be a polite live region so toasts are announced",
);
assert.ok(
  dropHint.includes('role="status"') || dropHint.includes('aria-live="polite"'),
  "DropHint must surface a status/live region",
);

// ---- F33 — chrome landmarks present, and reverted mistakes stay reverted ----
assert.ok(shell.includes("<nav") || shell.includes('role="navigation"'), "side chrome must expose a navigation landmark");
assert.ok(shell.includes('role="search"'), "search/filter surfaces must expose role=search");
assert.ok(shell.includes('role="tablist"'), "tab strip must expose role=tablist");
assert.ok(shell.includes('role="tab"'), "tabs must expose role=tab");
assert.ok(
  shell.includes(":aria-selected=") || shell.includes('aria-selected="'),
  "tabs must reflect the active tab via aria-selected",
);
// Reverts from F33 / F33b / F33c must not come back:
assert.ok(!shell.includes('role="tabpanel"'), "F33: role=tabpanel must stay removed from chrome tabs");
assert.ok(!shell.includes('role="region"'), "F33c: the body/main must not be wrapped in role=region");

console.log(
  "A11y semantics checks OK (F26/F34 listbox|grid derived roles + gridcell/columnheader/aria-sort, F27 tree/treeitem, F28 live region, F33 landmarks + tabpanel/region reverts).",
);
