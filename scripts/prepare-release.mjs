// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 本地发版准备：同步版本 →（可选）提交 → 打 annotated tag。
 * 推送 tag 后由 GitHub Actions 打包并创建 Release。
 *
 * Usage:
 *   node scripts/prepare-release.mjs 0.2.0
 *   node scripts/prepare-release.mjs 0.2.0 --commit
 *   node scripts/prepare-release.mjs 0.2.0 --commit --push
 */
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { isPrerelease, normalizeVersion, syncVersionFiles } from "./sync-version.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, {
    cwd: root,
    encoding: "utf8",
    stdio: opts.capture ? "pipe" : "inherit",
    ...opts,
  });
  if (r.status !== 0) {
    const err = (r.stderr || r.stdout || "").trim();
    throw new Error(err || `${cmd} ${args.join(" ")} failed`);
  }
  return (r.stdout || "").trim();
}

function main() {
  const args = process.argv.slice(2);
  const versionArg = args.find((a) => !a.startsWith("-"));
  const doCommit = args.includes("--commit");
  const doPush = args.includes("--push");

  if (!versionArg) {
    console.error("Usage: node scripts/prepare-release.mjs <version> [--commit] [--push]");
    process.exit(1);
  }

  const ver = normalizeVersion(versionArg);
  const tag = `v${ver}`;
  const pre = isPrerelease(ver);

  const status = run("git", ["status", "--porcelain"], { capture: true });
  if (status && !doCommit) {
    console.warn("工作区有未提交变更；将只改版本文件。建议干净后加 --commit。");
  }

  syncVersionFiles(ver);
  console.log(`已同步版本 → ${ver}（tag ${tag}${pre ? "，预发布" : ""}）`);

  if (doCommit) {
    run("git", ["add", "package.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml"]);
    // 允许空提交以外的 hook 失败直接抛错
    const staged = run("git", ["diff", "--cached", "--name-only"], { capture: true });
    if (!staged) {
      console.log("版本文件无变更，跳过 commit。");
    } else {
      run("git", ["commit", "-m", `chore(release): ${tag}`]);
    }
  }

  const existing = spawnSync("git", ["rev-parse", `refs/tags/${tag}`], {
    cwd: root,
    encoding: "utf8",
  });
  if (existing.status === 0) {
    throw new Error(`本地已存在 tag ${tag}，请先删除或换版本号`);
  }

  run("git", ["tag", "-a", tag, "-m", `Release ${tag}`]);
  console.log(`已创建 annotated tag: ${tag}`);

  if (doPush) {
    run("git", ["push", "origin", "HEAD"]);
    run("git", ["push", "origin", tag]);
    console.log(`已推送 HEAD 与 ${tag} → 将触发 GitHub Actions Release 工作流`);
  } else {
    console.log("");
    console.log("下一步：");
    console.log(`  git push origin HEAD && git push origin ${tag}`);
    console.log("推送 tag 后 Actions 会生成发版说明、打包 NSIS 并上传到 GitHub Release。");
  }
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
