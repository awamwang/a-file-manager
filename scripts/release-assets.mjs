// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 多包发版产物命名与发现（同一 semver / 同一 GitHub Release）。
 *
 * 约定资产名（空格→点，与历史 GitHub asset 一致）：
 *   - Core:         A.File.Manager_${ver}_x64-setup.exe
 *   - AI:           A.File.Manager_${ver}_x64-ai-setup.exe
 *   - Feature Pack: A.File.Manager_${ver}_feature-pack-${dir}.zip
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
  if (flavor === "ai") {
    return `${PRODUCT_ASSET_PREFIX}_${version}_x64-ai-setup.exe`;
  }
  if (flavor === "core") {
    return `${PRODUCT_ASSET_PREFIX}_${version}_x64-setup.exe`;
  }
  throw new Error(`未知 NSIS flavor: ${flavor}`);
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
