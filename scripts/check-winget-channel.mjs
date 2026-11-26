// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * ADR 0031：winget 包管理器分发渠道门禁。
 *
 * 静态针（脚本存在 + package.json 接线 + tauri.conf.json publisher 对齐）
 * + 真执行（buildWingetManifests / manifestRelativeDir / installerUrl 断言，
 * 含主线版素名资产、清单字段、目录布局）。
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  AFM_LICENSE,
  AFM_REPO,
  WINGET_MIN_OS_VERSION,
  WINGET_PACKAGE_IDENTIFIER,
  WINGET_PKGS_REPO,
  WINGET_PUBLISHER,
  assertPlainVersion,
  buildWingetManifests,
  installerUrl,
  manifestRelativeDir,
  wingetInstallerAssetName,
} from "./package-winget.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

// --- 接线（package.json） ---
const pkg = JSON.parse(read("package.json"));
assert.equal(pkg.scripts["package:winget"], "node scripts/package-winget.mjs");
assert.equal(pkg.scripts["test:winget-channel"], "node scripts/check-winget-channel.mjs");
assert.ok(
  pkg.scripts.test.includes("npm run test:winget-channel"),
  "test:winget-channel 必须接入聚合 test 链",
);

// --- 安装器元数据与脚本静态针 ---
const tauriConf = JSON.parse(read("src-tauri/tauri.conf.json"));
assert.equal(tauriConf.bundle.publisher, WINGET_PUBLISHER, "tauri.conf.json bundle.publisher 与清单 Publisher 对齐");

const script = read("scripts/package-winget.mjs");
assert.ok(script.includes("nullsoft"), "清单声明 InstallerType: nullsoft");
assert.ok(script.includes("Silent: /S"), "NSIS 静默安装开关 /S");
assert.ok(script.includes(WINGET_PKGS_REPO), "输出提交指引指向 microsoft/winget-pkgs");
assert.ok(script.includes("--dry-run"), "支持 --dry-run");

// --- 真执行：资产名 / URL / 目录布局 / 版本门 ---
assert.equal(
  wingetInstallerAssetName("0.4.0"),
  "A.File.Manager_0.4.0_x64-setup.exe",
  "winget 渠道只发主线版（素名，ADR 0024），不发 -lite",
);
assert.equal(
  installerUrl("0.4.0"),
  `https://github.com/${AFM_REPO}/releases/download/v0.4.0/A.File.Manager_0.4.0_x64-setup.exe`,
);
assert.equal(
  manifestRelativeDir(WINGET_PACKAGE_IDENTIFIER),
  "a/Awam/AFileManager",
  "winget-pkgs 目录布局：首字母小写独立成级",
);
assert.throws(() => assertPlainVersion("0.3.1-rc.6"), /预发布/, "winget 社区源拒绝预发布版本");
assert.doesNotThrow(() => assertPlainVersion("0.4.0"));

// --- 真执行：清单内容 ---
const sha = "A".repeat(64);
const manifests = buildWingetManifests({ version: "0.4.0", installerSha256: sha, releaseDate: "2026-12-01" });
assert.equal(manifests.length, 4, "version / installer / en-US / zh-CN 四文件");
assert.deepEqual(
  manifests.map((m) => m.fileName),
  [
    `${WINGET_PACKAGE_IDENTIFIER}.yaml`,
    `${WINGET_PACKAGE_IDENTIFIER}.installer.yaml`,
    `${WINGET_PACKAGE_IDENTIFIER}.locale.en-US.yaml`,
    `${WINGET_PACKAGE_IDENTIFIER}.locale.zh-CN.yaml`,
  ],
);
const all = manifests.map((m) => m.content).join("\n");
for (const needle of [
  `PackageIdentifier: ${WINGET_PACKAGE_IDENTIFIER}`,
  "PackageVersion: 0.4.0",
  `Publisher: ${WINGET_PUBLISHER}`,
  "InstallerType: nullsoft",
  `InstallerSha256: ${sha}`,
  "Silent: /S",
  `License: ${AFM_LICENSE}`,
  `MinimumOSVersion: ${WINGET_MIN_OS_VERSION}`,
  "ReleaseDate: 2026-12-01",
  "DefaultLocale: en-US",
]) {
  assert.ok(all.includes(needle), `清单缺少: ${needle}`);
}
assert.throws(() => buildWingetManifests({ version: "0.4.0", installerSha256: "xyz", releaseDate: "2026-12-01" }), /64/);

console.log(
  `winget-channel OK (ADR 0031: ${WINGET_PACKAGE_IDENTIFIER} @ ${WINGET_PKGS_REPO}, publisher=${WINGET_PUBLISHER}).`,
);
