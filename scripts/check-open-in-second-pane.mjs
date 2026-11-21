// SPDX-License-Identifier: GPL-3.0-or-later
// FIL-74: 标签右键 / 文件夹右键「在第二窗口打开」→ 双栏副栏的全链路静态契约。
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const state = read("src/state/dualPane/index.ts");
const model = read("src/components/navigator/contextMenuModel.ts");
const menu = read("src/components/navigator/NavigatorContextMenu.vue");
const shell = read("src/components/shell/AppShell.vue");
const pkg = JSON.parse(read("package.json"));

// 共享动作：只开不关；未开先打开双栏，已开只替换副栏路径。
assert.match(state, /export async function openInSecondPane\(path: string\)/);
assert.match(
  state,
  /dualPaneEnabled\.value = true;[\s\S]{0,160}await navigateSecondary\(target\)/,
  "helper must enable dual pane then navigate the secondary pane",
);
assert.ok(
  !/openInSecondPane[\s\S]{0,400}toggleDualPane\(/.test(state),
  "must never toggle an already-open dual pane off",
);
assert.ok(state.includes("if (!target) return;"), "empty path must be a no-op");

// 文件夹右键：目录专属顶层项 + 动作分发。
assert.ok(model.includes("secondPaneItems"));
assert.ok(model.includes("...secondPaneItems,"));
assert.ok(model.includes('id: "openInSecondPane"'));
assert.ok(model.includes('labelKey: "contextMenu.openInSecondPane"'));
assert.ok(model.includes("canOpenInSecondPane"));
assert.ok(menu.includes('case "openInSecondPane"'));
assert.ok(menu.includes("canOpenInSecondPane(entry)"));
assert.ok(menu.includes("await openInSecondPane(entry.path)"));
assert.match(
  menu,
  /import \{[^}]*openInSecondPane[^}]*\} from "\.\.\/\.\.\/state\/dualPane"/,
  "folder menu must import the shared helper",
);

// 标签右键：入口 / handler / 路径守卫 / 弹出高度。
assert.ok(shell.includes('t("chrome.openInSecondPane")'));
assert.ok(shell.includes("onOpenTabInSecondPane"));
assert.ok(shell.includes('@click="onOpenTabInSecondPane"'));
assert.ok(shell.includes("await openInSecondPane(path)"));
assert.ok(shell.includes(':disabled="!tabContextHasPath"'));
assert.ok(shell.includes("const menuHeight = 268;"));
assert.match(
  shell,
  /import \{[\s\S]*?openInSecondPane,[\s\S]*?\} from "\.\.\/\.\.\/state\/dualPane"/,
  "tab menu must import the shared helper",
);

// 5 语言：chrome（标签）与 contextMenu（文件夹）两个命名空间都要有。
for (const id of ["en", "zh-CN", "ja", "de", "fr"]) {
  const catalog = JSON.parse(read(`src/i18n/locales/${id}.json`));
  for (const ns of ["chrome", "contextMenu"]) {
    const label = catalog[ns]?.openInSecondPane;
    assert.equal(typeof label, "string", `${id}.${ns}.openInSecondPane`);
    assert.ok(label.trim().length > 0, `${id}.${ns}.openInSecondPane is empty`);
  }
}

assert.ok(pkg.scripts["test:open-in-second-pane"], "package.json must expose the check");
assert.ok(
  pkg.scripts.test.includes("test:open-in-second-pane"),
  "check must be part of the npm test chain",
);

const runtime = spawnSync(
  process.execPath,
  [
    "--experimental-strip-types",
    join(root, "scripts/open-in-second-pane-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (runtime.stdout) process.stdout.write(runtime.stdout);
if (runtime.stderr) process.stderr.write(runtime.stderr);
assert.equal(runtime.status, 0, "open-in-second-pane model checks failed");

console.log("Open in second pane checks OK (FIL-74).");
