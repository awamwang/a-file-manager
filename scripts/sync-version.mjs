// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 将语义化版本同步到 package.json / tauri.conf.json / Cargo.toml。
 *
 * Usage:
 *   node scripts/sync-version.mjs 0.2.0
 *   node scripts/sync-version.mjs v0.2.0
 *   node scripts/sync-version.mjs --from-tag   # 读当前 git 精确 tag（CI 用）
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

/** @param {string} raw */
export function normalizeVersion(raw) {
  const v = String(raw || "").trim().replace(/^v/i, "");
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(v)) {
    throw new Error(`非法版本号: ${raw}`);
  }
  return v;
}

/** @param {string} version */
export function isPrerelease(version) {
  return /-(?:alpha|beta|rc|pre|dev)(?:[.-]|$)/i.test(version) || /-\d/.test(version);
}

function read(rel) {
  return readFileSync(join(root, rel), "utf8");
}

function write(rel, text) {
  writeFileSync(join(root, rel), text, "utf8");
}

/** @param {string} version */
export function syncVersionFiles(version) {
  const ver = normalizeVersion(version);

  const pkgPath = "package.json";
  const pkg = JSON.parse(read(pkgPath));
  pkg.version = ver;
  write(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);

  const tauriPath = "src-tauri/tauri.conf.json";
  const tauri = JSON.parse(read(tauriPath));
  tauri.version = ver;
  write(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`);

  const cargoPath = "src-tauri/Cargo.toml";
  const cargo = read(cargoPath);
  const nextCargo = cargo.replace(
    /^(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
    `$1${ver}$2`,
  );
  if (nextCargo === cargo && !cargo.includes(`version = "${ver}"`)) {
    throw new Error("未能更新 src-tauri/Cargo.toml 的 version 字段");
  }
  write(cargoPath, nextCargo);

  return ver;
}

function versionFromExactTag() {
  const r = spawnSync("git", ["describe", "--tags", "--exact-match", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  });
  if (r.status !== 0) {
    throw new Error("当前 HEAD 不是精确 tag；CI 请在 push tags 触发，或传入显式版本");
  }
  return normalizeVersion(r.stdout.trim());
}

function main() {
  const arg = process.argv[2];
  if (!arg) {
    console.error("Usage: node scripts/sync-version.mjs <version|vX.Y.Z|--from-tag>");
    process.exit(1);
  }
  const ver = arg === "--from-tag" ? versionFromExactTag() : normalizeVersion(arg);
  const synced = syncVersionFiles(ver);
  console.log(`version synced → ${synced}`);
  console.log(`prerelease=${isPrerelease(synced)}`);
}

const entry = process.argv[1] ? pathToFileURL(process.argv[1]).href : "";
if (import.meta.url === entry) {
  try {
    main();
  } catch (e) {
    console.error(e instanceof Error ? e.message : e);
    process.exit(1);
  }
}
