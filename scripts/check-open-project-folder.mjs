// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-253: Project detail has a button that opens the Workspace folder on disk.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const projects = readFileSync(join(root, "src/views/ProjectsView.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));
const locales = ["en", "zh-CN", "ja", "de", "fr"];

assert.ok(projects.includes("async function openWorkspaceFolder"));
assert.ok(projects.includes('"open_path_default"'));
assert.ok(projects.includes("selected.value.path"));
assert.ok(projects.includes("@click=\"openWorkspaceFolder\"") || projects.includes("@click='openWorkspaceFolder'"));
assert.ok(projects.includes("projects.openWorkspaceFolder"));
assert.ok(projects.includes("projects.noticeOpenedWorkspace"));
assert.ok(projects.includes("projects.openWorkspaceFailed"));
// Prefer OS open of the Workspace path (not only IDE / member-link open).
assert.ok(projects.includes("openAllLinks"));
assert.ok(projects.includes("openWorkspaceFolder"));

const keys = [
  "openWorkspaceFolder",
  "openWorkspaceFolderHint",
  "noticeOpenedWorkspace",
  "openWorkspaceFailed",
];
for (const locale of locales) {
  const doc = JSON.parse(
    readFileSync(join(root, `src/i18n/locales/${locale}.json`), "utf8"),
  );
  for (const key of keys) {
    assert.equal(typeof doc.projects[key], "string", `${locale}.projects.${key}`);
    assert.ok(doc.projects[key].length > 0, `${locale}.projects.${key} empty`);
  }
}

assert.ok(en.projects.openWorkspaceFolder.toLowerCase().includes("workspace"));
assert.ok(
  zh.projects.openWorkspaceFolder.includes("工作区") ||
    zh.projects.openWorkspaceFolder.includes("文件夹"),
);

console.log("open-project-folder (AIA-253) OK.");
