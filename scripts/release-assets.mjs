// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 多包发版产物命名与发现（同一 semver / 同一 GitHub Release）。
 *
 * 版本定位（ADR 0024）：内部 flavor 标识符不变（core/ai），但面向用户的命名层反转——
 *   - `ai` flavor   = **主线版 Mainline**（默认含 ai-addon，= 普通正式版）→ 占用「素名」
 *   - `core` flavor = **精简版 Lite**（无 ai-addon）→ 带 `-lite` 标记后缀
 *
 * 约定资产名（空格→点，与历史 GitHub asset 一致）：
 *   - 主线版:        A.File.Manager_${ver}_x64-setup.exe
 *   - 精简版:        A.File.Manager_${ver}_x64-lite-setup.exe
 *   - Feature Pack: A.File.Manager_${ver}_feature-pack-${dir}.zip
 *   - 主线版 macOS:  A.File.Manager_${ver}_${arch}.dmg / .app.zip
 *   - 精简版 macOS:  A.File.Manager_${ver}_${arch}-lite.dmg / -lite.app.zip
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

export const PRODUCT_ASSET_PREFIX = "A.File.Manager";
export const APP_FLAVORS = ["core", "ai"];
export const ALL_PACK_KINDS = ["core", "ai", "feature-pack"];

/** Tauri NSIS 默认输出文件名（含空格） */
export function tauriNsisFileName(version) {
  return `A File Manager_${version}_x64-setup.exe`;
}

/** @param {string} version @param {"core"|"ai"} flavor */
export function nsisAssetName(version, flavor) {
  // 主线版 = ai flavor → 占用素名；精简版 = core flavor → `-lite` 后缀。见 ADR 0024。
  if (flavor === "ai") {
    return `${PRODUCT_ASSET_PREFIX}_${version}_x64-setup.exe`;
  }
  if (flavor === "core") {
    return `${PRODUCT_ASSET_PREFIX}_${version}_x64-lite-setup.exe`;
  }
  throw new Error(`未知 NSIS flavor: ${flavor}`);
}

/** macOS CI 构建矩阵：每个 flavor × 两个原生架构（7.25）。 */
export const MAC_ARCHES = [
  { target: "aarch64-apple-darwin", arch: "aarch64" },
  { target: "x86_64-apple-darwin", arch: "x64" },
];

/** Tauri DMG 默认输出文件名（含空格）：产品名_版本_架构.dmg */
export function tauriDmgFileName(version, arch) {
  if (!MAC_ARCHES.some((item) => item.arch === arch)) {
    throw new Error(`未知 macOS 架构: ${arch}`);
  }
  return `A File Manager_${version}_${arch}.dmg`;
}

/** GitHub Release 上的规范 DMG 资产名。 */
export function dmgAssetName(version, flavor, arch) {
  if (flavor !== "core" && flavor !== "ai") throw new Error(`未知 macOS flavor: ${flavor}`);
  tauriDmgFileName(version, arch);
  const suffix = flavor === "core" ? "-lite" : "";
  return `${PRODUCT_ASSET_PREFIX}_${version}_${arch}${suffix}.dmg`;
}

/** 与 DMG 同源的 .app 压缩包资产名（ditto -c -k --keepParent）。 */
export function appZipAssetName(version, flavor, arch) {
  if (flavor !== "core" && flavor !== "ai") throw new Error(`未知 macOS flavor: ${flavor}`);
  tauriDmgFileName(version, arch);
  const suffix = flavor === "core" ? "-lite" : "";
  return `${PRODUCT_ASSET_PREFIX}_${version}_${arch}${suffix}.app.zip`;
}

/** 供 release.yml 的 mac_matrix 使用；只保留 packs 选中的 flavor。 */
export function macReleaseMatrix(packKinds) {
  const kinds = Array.isArray(packKinds) ? packKinds : [];
  const rows = [];
  for (const flavor of APP_FLAVORS) {
    if (!kinds.includes(flavor)) continue;
    for (const { target, arch } of MAC_ARCHES) {
      rows.push({ flavor, vite_ai: flavor === "ai" ? "1" : "0", target, arch });
    }
  }
  return rows;
}

/** @param {string} version @param {string} packDirName packs/ 下目录名 */
export function featurePackAssetName(version, packDirName) {
  return `${PRODUCT_ASSET_PREFIX}_${version}_feature-pack-${packDirName}.zip`;
}

/** @param {string} raw */
export function parsePackKinds(raw) {
  const parts = String(raw || "all")
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  if (parts.length === 0 || parts.includes("all")) {
    return [...ALL_PACK_KINDS];
  }
  const unknown = parts.filter((p) => !ALL_PACK_KINDS.includes(p));
  if (unknown.length) {
    throw new Error(`未知包种类: ${unknown.join(", ")}（允许: ${ALL_PACK_KINDS.join("|")}|all）`);
  }
  return [...new Set(parts)];
}

/**
 * 发现可发布的 Feature Pack 目录（含 pack.json，排除 _template）。
 * @returns {{ dirName: string, absPath: string, manifest: object }[]}
 */
export function listFeaturePacks() {
  const packsRoot = join(root, "packs");
  if (!existsSync(packsRoot)) return [];
  const out = [];
  for (const name of readdirSync(packsRoot)) {
    if (name.startsWith("_") || name.startsWith(".")) continue;
    const abs = join(packsRoot, name);
    if (!statSync(abs).isDirectory()) continue;
    const manifestPath = join(abs, "pack.json");
    if (!existsSync(manifestPath)) continue;
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    out.push({ dirName: name, absPath: abs, manifest });
  }
  return out.sort((a, b) => a.dirName.localeCompare(b.dirName));
}

export function releaseDistDir() {
  return join(root, "dist", "release");
}

export function repoRoot() {
  return root;
}
