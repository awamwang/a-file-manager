// SPDX-License-Identifier: GPL-3.0-or-later
// Work Groups / Projects: path cards, usage hints, unified edit chrome.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const wg = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
const projects = readFileSync(join(root, "src/views/ProjectsView.vue"), "utf8");
const pathCards = readFileSync(join(root, "src/styles/path-cards.css"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(wg.includes("path-card-grid"));
assert.ok(wg.includes("openOrFocusNavigatorTab"));
assert.ok(wg.includes("workGroups.usageHint"));
assert.ok(projects.includes("path-card-grid"));
assert.ok(projects.includes("openLink"));
assert.ok(projects.includes("projects.usageHint"));
assert.ok(en.workGroups.usageHint.length > 10);
assert.ok(en.projects.usageHint.length > 10);
assert.ok(zh.workGroups.usageHint.includes("标签") || zh.workGroups.usageHint.length > 8);
assert.ok(zh.projects.usageHint.includes("项目") || zh.projects.usageHint.length > 8);

// Unified edit chrome: ⋯ only (no standalone delete tag), right-aligned.
assert.ok(!wg.includes("name-tag--danger"));
assert.ok(!projects.includes("name-tag--danger"));
assert.ok(wg.includes('class="more-menu"') || wg.includes("class=\"more-menu\"") || wg.includes("more-menu"));
assert.ok(projects.includes("more-menu"));
assert.ok(wg.includes("margin-inline-start: auto"));
assert.ok(projects.includes("margin-inline-start: auto"));

// Master-list item ⋯ (rename + delete) + independent list scroll.
assert.ok(wg.includes("list-more-menu"));
assert.ok(projects.includes("list-more-menu"));
assert.ok(wg.includes("startListRename"));
assert.ok(projects.includes("startListRename"));
assert.ok(wg.includes("commitListRename"));
assert.ok(projects.includes("commitListRename"));
assert.ok(wg.includes("deleteGroup"));
assert.ok(projects.includes("deleteProject"));
assert.ok(wg.includes("master-list-scroll"));
assert.ok(projects.includes("master-list-scroll"));
assert.match(wg, /\.master-list-scroll\s*\{[^}]*overflow-y:\s*auto/s);
assert.match(projects, /\.master-list-scroll\s*\{[^}]*overflow-y:\s*auto/s);
// AIA-255: list/detail ⋯ menus Teleport to body (not clipped by fluent-surface overflow).
assert.ok(wg.includes('<Teleport to="body">'));
assert.ok(projects.includes('<Teleport to="body">'));
assert.ok(wg.includes("more-popover--portal"));
assert.ok(projects.includes("more-popover--portal"));
assert.ok(wg.includes("positionMorePopover"));
assert.ok(projects.includes("positionMorePopover"));

// Inline rename: Enter saves; blur/Escape cancel without save; toast on save.
assert.ok(wg.includes('@keydown.enter.prevent="commitInlineRename"'));
assert.ok(wg.includes('@keydown.escape.prevent="cancelInlineRename"'));
assert.ok(wg.includes('@blur="onRenameBlur"'));
assert.ok(wg.includes("onRenameBlur"));
assert.ok(!wg.includes('@blur="commitInlineRename"'));
assert.ok(wg.includes("toastSuccess(t(\"workGroups.renamed\"))") || wg.includes("toastSuccess(t('workGroups.renamed'))"));
assert.ok(projects.includes('@keydown.enter.prevent="commitProjectInlineRename"'));
assert.ok(projects.includes('@keydown.escape.prevent="cancelProjectInlineRename"'));
assert.ok(projects.includes('@blur="onProjectRenameBlur"'));
assert.ok(projects.includes("onProjectRenameBlur"));
assert.ok(!projects.includes('@blur="commitProjectInlineRename"'));
assert.ok(
  projects.includes("toastSuccess(t(\"projects.renamed\"))") ||
    projects.includes("toastSuccess(t('projects.renamed'))"),
);

// Click-away closes menus / cancels rename.
assert.ok(wg.includes("pointerdown"));
assert.ok(projects.includes("pointerdown"));
assert.ok(wg.includes("onDetailPointerDown"));
assert.ok(projects.includes("onProjectDetailPointerDown"));

// Member/link remove is trailing chrome on the card row (not a stacked full-width row).
assert.match(pathCards, /\.path-card--with-action\s*\{[^}]*flex-direction:\s*row/s);
assert.ok(!/flex-direction:\s*column/.test(pathCards.match(/\.path-card--with-action\s*\{[^}]*\}/s)?.[0] ?? ""));
assert.ok(wg.includes('name="close"'));
assert.ok(projects.includes('name="close"'));
assert.ok(wg.includes("@click.stop=\"removeMember"));
assert.ok(projects.includes("@click.stop=\"removeLink"));

// AIA-274: paths are copyable (button + Teleport context menu).
assert.ok(wg.includes("copyMemberPath"));
assert.ok(wg.includes("openPathMenu"));
assert.ok(wg.includes('name="copy"') || wg.includes("name='copy'"));
assert.ok(wg.includes("contextMenu.copyPath"));
assert.ok(wg.includes("pathMenuPath"));
assert.ok(projects.includes("copyLinkPath"));
assert.ok(projects.includes("openPathMenu"));
assert.ok(projects.includes("contextMenu.copyPath"));
assert.ok(projects.includes("pathMenuPath"));
assert.ok(pathCards.includes("path-card-copy"));
assert.ok(pathCards.includes("user-select: text"));
assert.equal(en.contextMenu.copyPath, "Copy path");
assert.ok(zh.contextMenu.copyPath.includes("路径") || zh.contextMenu.copyPath.includes("複製"));

// FIL-8: 长路径完整可见（换行，不 ellipsis 截断）。
assert.match(pathCards, /\.path-card-path\s*\{[^}]*overflow-wrap:\s*anywhere/s);
assert.match(pathCards, /\.path-card-path\s*\{[^}]*white-space:\s*normal/s);
assert.ok(!/\.path-card-path\s*\{[^}]*text-overflow:\s*ellipsis/s.test(pathCards));
assert.ok(!/\.path-card-path\s*\{[^}]*white-space:\s*nowrap/s.test(pathCards));
assert.match(
  pathCards,
  /\.path-card-grid\s*\{[^}]*minmax\(min\(100%,\s*22rem\),\s*1fr\)/s,
);

console.log("wg/project path cards + list ⋯ + scroll chrome OK.");
