// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 将 packs/ 下各 Feature Pack 打成可安装 zip，写入 dist/release/。
 *
 * Usage:
 *   node scripts/package-feature-packs.mjs [version]
 *   node scripts/package-feature-packs.mjs 0.3.0 --dry-run
 *   node scripts/package-feature-packs.mjs --from-package
 */
import { mkdirSync, rmSync, existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import {
  featurePackAssetName,
  listFeaturePacks,
  releaseDistDir,
  repoRoot,
} from "./release-assets.mjs";
import { normalizeVersion } from "./sync-version.mjs";

const root = repoRoot();

function run(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: root, encoding: "utf8", shell: false });
  if (r.status !== 0) {
    throw new Error((r.stderr || r.stdout || `${cmd} failed`).trim());
  }
  return (r.stdout || "").trim();
}

/** 用 tar 生成 zip（Windows bsdtar / GitHub runners 可用） */
function zipPackDir(absPackDir, outZip) {
  if (existsSync(outZip)) rmSync(outZip);
  // -a: 按扩展名选格式；-C 后路径为包根，zip 内含 pack.json
  run("tar", ["-a", "-cf", outZip, "-C", absPackDir, "."]);
}

/**
 * @param {{ version?: string, dryRun?: boolean }} [opts]
 * @returns {{ file: string, packId: string, packVersion: string }[]}
 */
export function packageFeaturePacks(opts = {}) {
  const dryRun = Boolean(opts.dryRun);
  const ver = normalizeVersion(
    opts.version ||
      JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version,
  );
  const packs = listFeaturePacks();
  const outDir = releaseDistDir();

  if (!packs.length) {
    console.warn("未发现可打包的 Feature Pack（packs/*/pack.json，排除 _*）");
    return [];
  }

  if (!dryRun) mkdirSync(outDir, { recursive: true });

  /** @type {{ file: string, packId: string, packVersion: string }[]} */
  const written = [];
  for (const p of packs) {
    const file = featurePackAssetName(ver, p.dirName);
    const outZip = join(outDir, file);
    console.log(
      `${dryRun ? "[dry-run] " : ""}Feature Pack ${p.manifest.id}@${p.manifest.version} → ${file}`,
    );
    if (!dryRun) {
      zipPackDir(p.absPath, outZip);
    }
    written.push({
      file,
      packId: String(p.manifest.id || p.dirName),
      packVersion: String(p.manifest.version || ""),
    });
  }

  console.log(
    dryRun
      ? `dry-run: 将生成 ${written.length} 个 zip（版本戳 ${ver}）`
      : `已写入 ${written.length} 个 Feature Pack zip → ${outDir}`,
  );
  return written;
}

function main() {
  const args = process.argv.slice(2);
  const dryRun = args.includes("--dry-run");
  let version;
  if (args.includes("--from-package")) {
    version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
  } else {
    version = args.find((a) => !a.startsWith("-"));
  }
  packageFeaturePacks({ version, dryRun });
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
