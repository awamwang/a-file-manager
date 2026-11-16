// SPDX-License-Identifier: GPL-3.0-or-later
/** macOS 打包契约：bundle targets / 运行时资源 / 最低系统版本，且不回退 Windows 契约。 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");
const conf = JSON.parse(read("src-tauri/tauri.conf.json"));
const bundle = conf.bundle ?? {};

// P0-6：macOS 的 .app / .dmg 与 Windows 的 NSIS 共存；Windows 发布仍显式指定 nsis。
const targets = Array.isArray(bundle.targets)
  ? bundle.targets.map((value) => String(value).toLowerCase())
  : [];
for (const target of ["nsis", "app", "dmg"]) {
  assert.ok(targets.includes(target), `bundle.targets must include ${target}`);
}
assert.equal(bundle.active, true, "bundling must stay enabled");
assert.ok(
  read("scripts/build-flavor.mjs").includes('"nsis"'),
  "Windows release must keep explicit --bundles nsis",
);

// 拖出预览图标走 resolveResource：不随包分发就会「dev 有图标、release 没有」。
const dragIcon = "icons/32x32.png";
const resources = Array.isArray(bundle.resources) ? bundle.resources : [];
assert.ok(resources.includes(dragIcon), `bundle.resources must ship ${dragIcon} for drag-out preview`);
assert.ok(existsSync(join(root, "src-tauri", dragIcon)), `${dragIcon} must exist`);
const dragOut = read("src/state/dnd/dragOut.ts");
const resolved = /resolveResource\(\s*["']([^"']+)["']\s*\)/.exec(dragOut)?.[1];
assert.equal(resolved, dragIcon, "resolveResource path must match bundled resource");

// .app 需要 icns；10.13 是 Tauri 默认值，这里显式钉 10.15（Tauri 2 / WKWebView 实际下限）。
assert.ok(bundle.icon?.includes("icons/icon.icns"), "macOS .app needs icon.icns");
assert.ok(existsSync(join(root, "src-tauri/icons/icon.icns")), "icons/icon.icns must exist");
assert.equal(
  bundle.macOS?.minimumSystemVersion,
  "10.15",
  "pin macOS minimumSystemVersion to 10.15",
);

// 增加平台 target 不得以牺牲既有 Windows 发布 / 窗口契约为代价。
assert.equal(
  bundle.windows?.webviewInstallMode?.type,
  "embedBootstrapper",
  "NSIS WebView2 bootstrapper must stay",
);
assert.equal(conf.app?.windows?.[0]?.decorations, false, "custom Fluent caption must stay");

console.log("macos-bundle: ok");
