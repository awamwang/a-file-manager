// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 本地打齐某一版本的多包产物到 dist/release/（不打 tag、不上传）。
 *
 * Usage:
 *   node scripts/build-release-artifacts.mjs [--version 0.3.0] [--packs all|core,ai,feature-pack]
 *   node scripts/build-release-artifacts.mjs --dry-run
 *   node scripts/build-release-artifacts.mjs --packs feature-pack
 */
import { copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import {
  APP_FLAVORS,
  nsisAssetName,
  parsePackKinds,
  releaseDistDir,
  repoRoot,
  tauriNsisFileName,
} from "./release-assets.mjs";
import { normalizeVersion, syncVersionFiles } from "./sync-version.mjs";
import { packageFeaturePacks } from "./package-feature-packs.mjs";

const root = repoRoot();

function run(cmd, args, env = process.env) {
  console.log(`> ${cmd} ${args.join(" ")}`);
  const r = spawnSync(cmd, args, { cwd: root, env, stdio: "inherit", shell: true });
  if (r.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} failed (exit ${r.status ?? 1})`);
  }
}

function parseArgs(argv) {
  const out = {
    version: null,
    packs: "all",
    dryRun: false,
    syncVersion: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--dry-run") out.dryRun = true;
    else if (a === "--sync-version") out.syncVersion = true;
    else if (a === "--version" || a === "-v") {
      out.version = argv[++i];
    } else if (a === "--packs") {
      out.packs = argv[++i];
    } else if (!a.startsWith("-") && !out.version) {
      out.version = a;
    } else if (a.startsWith("-")) {
      throw new Error(`未知参数: ${a}`);
    }
  }
  return out;
}

function currentVersion() {
  return normalizeVersion(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version);
}

function buildNsisFlavor(flavor, version, outDir) {
  run("node", ["scripts/build-flavor.mjs", flavor, "--bundle"]);
  const produced = join(
    root,
    "src-tauri",
    "target",
    "release",
    "bundle",
    "nsis",
    tauriNsisFileName(version),
  );
  if (!existsSync(produced)) {
    throw new Error(`未找到 NSIS 产物: ${produced}`);
  }
  const destName = nsisAssetName(version, flavor);
  const dest = join(outDir, destName);
  copyFileSync(produced, dest);
  console.log(`已收集 ${flavor} → ${dest}`);
  return destName;
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  const kinds = parsePackKinds(opts.packs);
  const ver = normalizeVersion(opts.version || currentVersion());
  const outDir = releaseDistDir();

  console.log(`多包产物版本=${ver} packs=${kinds.join(",")} dryRun=${opts.dryRun}`);

  if (opts.dryRun) {
    if (kinds.includes("core")) {
      console.log(`[dry-run] NSIS core → ${nsisAssetName(ver, "core")}`);
    }
    if (kinds.includes("ai")) {
      console.log(`[dry-run] NSIS ai → ${nsisAssetName(ver, "ai")}`);
    }
    if (kinds.includes("feature-pack")) {
      packageFeaturePacks({ version: ver, dryRun: true });
    }
    console.log("dry-run 完成（未构建、未改版本文件）");
    return;
  }

  if (opts.syncVersion) {
    syncVersionFiles(ver);
    console.log(`已同步版本文件 → ${ver}`);
  } else if (currentVersion() !== ver) {
    console.warn(
      `警告: package.json 版本为 ${currentVersion()}，产物将按 ${ver} 命名；如需写入版本源请加 --sync-version`,
    );
  }

  mkdirSync(outDir, { recursive: true });
  const artifacts = [];

  for (const flavor of APP_FLAVORS) {
    if (!kinds.includes(flavor)) continue;
    // 每种 flavor 构建后立即拷贝；下一次构建会覆盖同名 Tauri 输出
    artifacts.push(buildNsisFlavor(flavor, ver, outDir));
  }

  if (kinds.includes("feature-pack")) {
    packageFeaturePacks({ version: ver });
  }

  console.log("");
  console.log(`产物目录: ${outDir}`);
  console.log("下一步（可选本地上传到已有 draft/release）：");
  console.log(`  gh release upload v${ver} dist/release/* --clobber`);
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
