// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-281: soft Reload Window — titlebar control, shell remount, volatile cache clear. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const reload = read("src/state/uiReload.ts");
const app = read("src/App.vue");
const shell = read("src/components/shell/AppShell.vue");
const registry = read("src/commands/registry.ts");
const en = JSON.parse(read("src/i18n/locales/en.json"));
const zh = JSON.parse(read("src/i18n/locales/zh-CN.json"));

assert.ok(reload.includes("export async function reloadWindow"), "must export reloadWindow");
assert.ok(reload.includes("export const shellRemountKey"), "must export shellRemountKey");
assert.ok(reload.includes("clearShellIconQueue"), "must clear shell icon queue");
assert.ok(reload.includes("closeCommandPanel"), "must close Command Panel");
assert.ok(reload.includes("invalidateSettingsCache"), "must refresh settings cache from disk");
assert.ok(reload.includes("hydrateSettings"), "must re-hydrate settings");
assert.ok(reload.includes("refreshActiveListing"), "must re-bind listing after remount");
assert.ok(
  !/location\.reload|webview\.reload|relaunch/i.test(reload),
  "prefer soft remount; no hard location/webview reload in uiReload",
);

assert.ok(app.includes("shellRemountKey"), "App.vue must key shell on remount");
assert.ok(
  /AppShell\s*:key="shellRemountKey"/.test(app) ||
    /:key="shellRemountKey"/.test(app),
  "AppShell must remount via shellRemountKey",
);
assert.ok(app.includes("KeepAlive"), "KeepAlive remains (cleared by shell remount)");

assert.ok(shell.includes("titlebar-leading"), "reload control on left of titlebar");
assert.ok(shell.includes("titlebar-reload-window"), "stable test id for titlebar reload");
assert.ok(shell.includes("reloadWindow"), "AppShell wires reloadWindow");
assert.ok(
  /t\(["']chrome\.reloadWindow["']\)/.test(shell),
  "titlebar aria uses chrome.reloadWindow",
);
{
  const header = shell.slice(shell.indexOf("<header"), shell.indexOf("</header>"));
  const leading = header.indexOf("titlebar-leading");
  const drag = header.indexOf("titlebar-drag");
  const actions = header.indexOf("titlebar-actions");
  assert.ok(leading >= 0 && drag > leading, "leading reload must be left of drag region");
  assert.ok(actions > drag, "commands stay right of drag region");
  assert.ok(
    !header.slice(leading, leading + 400).includes("data-tauri-drag-region"),
    "reload button must not be a drag region",
  );
}

assert.ok(registry.includes("reloadWindow"), "command registry accepts reloadWindow dep");
assert.ok(registry.includes('"action.reload-window"') || registry.includes("action.reload-window"),
  "Command Panel must expose action.reload-window");
assert.ok(
  /t\(["']commands\.reloadWindow["']\)/.test(registry),
  "command title is i18n",
);

assert.equal(en.chrome.reloadWindow, "Reload Window");
assert.equal(zh.chrome.reloadWindow, "重新加载窗口");
assert.equal(en.commands.reloadWindow, "Reload Window");
assert.equal(zh.commands.reloadWindow, "重新加载窗口");
assert.ok(en.chrome.reloadWindowTitle);
assert.ok(zh.chrome.reloadWindowTitle);

console.log("ui-reload-window: ok");
