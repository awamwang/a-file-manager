// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 生成 winget 社区源（microsoft/winget-pkgs）清单并给出提交指引（ADR 0032）。
 *
 * 用法：npm run package:winget [-- --version 0.4.0] [--dry-run] [--release-date 2026-12-01]
 *
 * 约定：
 *   - 只发布**主线版** Windows NSIS 安装器（A.File.Manager_${ver}_x64-setup.exe，
 *     复用 release-assets.mjs 的 `nsisAssetName(ver, "ai")`——主线版占用素名，见 ADR 0024）。
 *   - PackageIdentifier = Awam.AFileManager，Publisher = Awam（与 tauri.conf.json
 *     `bundle.publisher` 对齐；此前安装器 CompanyName 为空，v0.4.0 起补齐）。
 *   - 站点展示走 installMethods 命令卡而非 downloadSources（下载源专指文件直链）。
 *   - winget 社区源不接受预发布版本，version 必须是纯 x.y.z。
 *
 * 产出：dist/winget/manifests/a/Awam/AFileManager/${version}/ 下 4 个清单文件
 * （version / installer / locale.en-US / locale.zh-CN）。提交 PR 到 winget-pkgs 由人工
 * 确认执行（不引入 bot / secret），步骤见脚本输出。
 */
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";
import { nsisAssetName } from "./release-assets.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

export const WINGET_PACKAGE_IDENTIFIER = "Awam.AFileManager";
export const WINGET_PUBLISHER = "Awam";
export const WINGET_MANIFEST_VERSION = "1.28.0";
/** Windows 10 1809（winget 最低系统要求） */
export const WINGET_MIN_OS_VERSION = "10.0.17763.0";
export const WINGET_PKGS_REPO = "microsoft/winget-pkgs";
export const AFM_REPO = "awamwang/a-file-manager";
export const AFM_LICENSE = "GPL-3.0-or-later";

/** 主线版 NSIS 资产名（单一事实源：release-assets.mjs，ADR 0024 素名约定）。 */
export function wingetInstallerAssetName(version) {
  return nsisAssetName(version, "ai");
}

/** GitHub Release 主线版资产直链（winget 清单 InstallerUrl）。 */
export function installerUrl(version) {
  return `https://github.com/${AFM_REPO}/releases/download/v${version}/${wingetInstallerAssetName(version)}`;
}

/** winget-pkgs 仓库内清单目录（首字母小写独立成级）：Awam.AFileManager → a/Awam/AFileManager */
export function manifestRelativeDir(identifier) {
  const parts = String(identifier).split(".");
  if (parts.length < 2 || parts.some((p) => !p)) {
    throw new Error(`非法 PackageIdentifier: ${identifier}`);
  }
  return [parts[0][0].toLowerCase(), ...parts].join("/");
}

/** winget PackageVersion 只接受 x.y.z[.w]，不接受预发布后缀。 */
export function assertPlainVersion(version) {
  if (!/^\d+(\.\d+){0,3}$/.test(version)) {
    throw new Error(
      `winget 社区源不接受预发布/带后缀版本: ${version}（须为纯 x.y.z，且只提交正式 Release）`,
    );
  }
}

/**
 * 构建 winget 多文件清单（schema 1.6.0）。
 * @param {{ version: string, installerSha256: string, releaseDate: string }} input
 * @returns {{ fileName: string, content: string }[]}
 */
export function buildWingetManifests({ version, installerSha256, releaseDate }) {
  assertPlainVersion(version);
  const id = WINGET_PACKAGE_IDENTIFIER;
  const url = installerUrl(version);
  const sha = String(installerSha256 || "").toUpperCase();
  if (!/^[0-9A-F]{64}$/.test(sha)) {
    throw new Error(`InstallerSha256 必须是 64 位十六进制: ${installerSha256}`);
  }
  const header = (type) =>
    `# yaml-language-server: $schema=https://aka.ms/winget-manifest.${type}.${WINGET_MANIFEST_VERSION}.schema.json`;
  const tag = `v${version}`;

  const versionManifest = `${header("version")}
PackageIdentifier: ${id}
PackageVersion: ${version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: ${WINGET_MANIFEST_VERSION}
`;

  const installerManifest = `${header("installer")}
PackageIdentifier: ${id}
PackageVersion: ${version}
MinimumOSVersion: ${WINGET_MIN_OS_VERSION}
Scope: user
InstallModes:
- interactive
- silent
UpgradeBehavior: install
ReleaseDate: ${releaseDate}
Installers:
- Architecture: x64
  InstallerType: nullsoft
  InstallerUrl: ${url}
  InstallerSha256: ${sha}
  InstallerSwitches:
    Silent: /S
    SilentWithProgress: /S
ManifestType: installer
ManifestVersion: ${WINGET_MANIFEST_VERSION}
`;

  const localeEn = `${header("defaultLocale")}
PackageIdentifier: ${id}
PackageVersion: ${version}
PackageLocale: en-US
PackageName: A File Manager
Publisher: ${WINGET_PUBLISHER}
Moniker: afilemanager
License: ${AFM_LICENSE}
LicenseUrl: https://github.com/${AFM_REPO}/blob/master/LICENSE
ShortDescription: Windows-first dual-pane file manager with work groups, projects and global search, plus an optional AI add-on.
Description: A File Manager is a modern desktop file manager (Tauri 2 + Vue 3, Fluent UI). It offers dual-pane browsing, tabs, global search, virtual file systems (rclone / awam-file gateway), and persists your working context as work groups and projects. An optional AI add-on ships an MCP and Skills hub that deploys to AI clients such as Cursor and Claude Code in one click.
PackageUrl: https://github.com/${AFM_REPO}
PublisherUrl: https://github.com/awamwang
PublisherSupportUrl: https://github.com/${AFM_REPO}/issues
Tags:
- file-manager
- dual-pane
- files
- windows
- tauri
ReleaseNotesUrl: https://github.com/${AFM_REPO}/releases/tag/${tag}
ManifestType: defaultLocale
ManifestVersion: ${WINGET_MANIFEST_VERSION}
`;

  const localeZh = `${header("locale")}
PackageIdentifier: ${id}
PackageVersion: ${version}
PackageLocale: zh-CN
PackageName: A File Manager
Publisher: ${WINGET_PUBLISHER}
License: ${AFM_LICENSE}
LicenseUrl: https://github.com/${AFM_REPO}/blob/master/LICENSE
ShortDescription: Windows 优先的双栏文件管理器：工作组 / 项目持久保留工作现场，Discovery 全局搜索，可选 AI 附加包。
Description: A File Manager 是基于 Tauri 2 + Vue 3 的现代桌面文件管理器（Fluent 原生界面）：双栏浏览、多标签、全局搜索 Discovery，支持 rclone / awam-file 网关等虚拟文件系统；「工作组 / 项目」把工作现场持久保留、一键切回。可选 AI 包内置 MCP 与 Skills 中央仓库，一键部署到 Cursor、Claude Code 等 AI 客户端。
PackageUrl: https://github.com/${AFM_REPO}
PublisherUrl: https://github.com/awamwang
PublisherSupportUrl: https://github.com/${AFM_REPO}/issues
Tags:
- file-manager
- dual-pane
- 文件管理器
ReleaseNotesUrl: https://github.com/${AFM_REPO}/releases/tag/${tag}
ManifestType: locale
ManifestVersion: ${WINGET_MANIFEST_VERSION}
`;

  return [
    { fileName: `${id}.yaml`, content: versionManifest },
    { fileName: `${id}.installer.yaml`, content: installerManifest },
    { fileName: `${id}.locale.en-US.yaml`, content: localeEn },
    { fileName: `${id}.locale.zh-CN.yaml`, content: localeZh },
  ];
}

/** 下载安装器并计算 SHA-256（主线版 ~5MB，全量进内存）。 */
export async function fetchInstallerSha256(url) {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`下载失败 HTTP ${res.status}: ${url}（Release 是否已发布该资产？）`);
  }
  const buf = Buffer.from(await res.arrayBuffer());
  return {
    sha256: createHash("sha256").update(buf).digest("hex").toUpperCase(),
    size: buf.length,
  };
}

function currentVersion() {
  return JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8")).version;
}

function parseArgs(argv) {
  const out = { dryRun: false };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === "--dry-run") out.dryRun = true;
    else if (a === "--version") out.version = argv[++i];
    else if (a === "--release-date") out.releaseDate = argv[++i];
    else if (a === "--out") out.out = argv[++i];
    else throw new Error(`未知参数: ${a}`);
  }
  return out;
}

function today() {
  return new Date().toISOString().slice(0, 10);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const version = args.version || currentVersion();
  assertPlainVersion(version);
  const releaseDate = args.releaseDate || today();
  const outRoot = join(root, args.out || join("dist", "winget"));
  const manifestDirRel = join(manifestRelativeDir(WINGET_PACKAGE_IDENTIFIER), version);

  let sha;
  if (args.dryRun) {
    console.log(`[dry-run] 跳过下载：${installerUrl(version)}`);
    sha = "0".repeat(64); // 占位，仅用于本地预览，不落盘
  } else {
    const url = installerUrl(version);
    console.log(`下载 ${url} ...`);
    const { sha256, size } = await fetchInstallerSha256(url);
    console.log(`SHA-256: ${sha256}（${(size / 1024 / 1024).toFixed(1)} MiB）`);
    sha = sha256;
  }

  const manifests = buildWingetManifests({ version, installerSha256: sha, releaseDate });
  const targetDir = join(outRoot, "manifests", manifestDirRel);
  console.log(`\n清单（${manifests.length} 个文件）→ ${join("dist", "winget", "manifests", manifestDirRel)}`);
  if (args.dryRun) {
    for (const m of manifests) console.log(`  [dry-run] ${m.fileName}（${m.content.length} 字节）`);
    console.log("\n[dry-run] 未写盘。正式执行：npm run package:winget");
    return;
  }

  mkdirSync(targetDir, { recursive: true });
  for (const m of manifests) writeFileSync(join(targetDir, m.fileName), m.content, "utf8");

  // winget validate（本机无 winget 时跳过；winget 退出码不可靠——成功也可能带非零码
  // 如 -1978335192，故解析输出文本判定）
  const validate = spawnSync(`winget validate --manifest "${targetDir}"`, {
    shell: true,
    encoding: "utf8",
  });
  const validateOut = `${validate.stdout || ""}${validate.stderr || ""}`;
  if (/Manifest validation failed/i.test(validateOut)) {
    console.error(validateOut.trim());
    throw new Error("winget validate 未通过——请按上方错误修正清单。");
  } else if (/Manifest validation succeeded/i.test(validateOut)) {
    console.log(`winget validate: OK${/warning/i.test(validateOut) ? "（含 warning，见上方输出）" : ""}`);
  } else {
    console.warn("⚠ 本机无 winget 或校验不可用——请人工核对清单（或用 winget-pkgs PR 流水线验证）。");
  }

  console.log(`
提交步骤（人工确认后执行；winget-pkgs 审核数天~数周）：
  1. fork ${WINGET_PKGS_REPO}（已有则跳过），克隆并同步 upstream/master
  2. git checkout -b add-afm-${version} upstream/master
  3. 复制清单目录 ${targetDir} → winget-pkgs/manifests/${manifestDirRel}
     （目录下只应有本版本的 ${manifests.length} 个 .yaml）
  4. git add manifests && git commit -m "Add ${WINGET_PACKAGE_IDENTIFIER} version ${version}"
  5. push 到 fork，向 ${WINGET_PKGS_REPO}:master 开 PR（标题同 commit message）
过审后：awam-software 合并 installMethods 分支（content/products/a-file-manager.json）并部署站点。`);
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main().catch((err) => {
    console.error(`✗ ${err.message}`);
    process.exit(1);
  });
}
