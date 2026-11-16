// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 7.27: finish the zero-credential macOS fallback.
 *
 * `tauri build` without APPLE_* leaves the .app completely unsigned
 * (`code object is not signed at all`). That makes Gatekeeper's
 * right-click → Open path unreliable on newer macOS versions. This script:
 *
 *   1. ad-hoc signs the .app (integrity seal, NOT Developer ID / notarization);
 *   2. rebuilds the Tauri .dmg with an in-package 打开帮助.txt at the root
 *      plus the usual /Applications drop target;
 *   3. verifies both the signature and the DMG image.
 *
 * Signed/notarized builds must NOT call this: repackaging would invalidate
 * the Developer ID signature. Their help file comes from bundle.resources and
 * is signed by Tauri together with the app.
 *
 * Usage:
 *   node scripts/finalize-macos-adhoc.mjs \
 *     --app "src-tauri/target/<triple>/release/bundle/macos/A File Manager.app" \
 *     --dmg "src-tauri/target/<triple>/release/bundle/dmg/A File Manager_<ver>_<arch>.dmg" \
 *     --help-file src-tauri/resources/macos-open-help.txt
 */
import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

const args = process.argv.slice(2);

function arg(name) {
  const i = args.indexOf(name);
  if (i < 0 || !args[i + 1] || args[i + 1].startsWith("--")) {
    throw new Error(`缺少参数 ${name}`);
  }
  return args[i + 1];
}

const appPath = arg("--app");
const dmgPath = arg("--dmg");
const helpPath = arg("--help-file");

if (process.platform !== "darwin") {
  throw new Error("ad-hoc 收尾只能在 macOS runner 上执行");
}
if (!existsSync(appPath)) throw new Error(`找不到 .app: ${appPath}`);
if (!existsSync(dmgPath)) throw new Error(`找不到 .dmg: ${dmgPath}`);
if (!existsSync(helpPath)) throw new Error(`找不到打开帮助文件: ${helpPath}`);

const embeddedHelp = join(appPath, "Contents", "Resources", "resources", "macos-open-help.txt");
if (!existsSync(embeddedHelp)) {
  throw new Error(
    `App 包内缺少打开帮助: ${embeddedHelp}（请检查 tauri.conf.json 的 bundle.resources）`,
  );
}

function run(cmd, cmdArgs) {
  console.log(`> ${cmd} ${cmdArgs.join(" ")}`);
  execFileSync(cmd, cmdArgs, { stdio: "inherit" });
}

function capture(cmd, cmdArgs) {
  // `codesign -dv` writes its details to stderr, so both streams matter.
  const r = spawnSync(cmd, cmdArgs, { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`${cmd} failed: ${r.stderr || r.stdout || r.error}`);
  return `${r.stdout || ""}\n${r.stderr || ""}`;
}

// 1. Explicit ad-hoc signature. `--timestamp=none` avoids any network call;
//    this is intentionally not a Developer ID signature.
run("codesign", ["--force", "--deep", "--sign", "-", "--timestamp=none", appPath]);
const info = capture("codesign", ["-dv", "--verbose=4", appPath]);
if (!/Signature=adhoc/.test(info)) {
  throw new Error(`ad-hoc 签名未生效: ${appPath}`);
}
run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath]);

// 2. Rebuild the DMG from the signed app plus a visible root-level help file.
//    Tauri's original DMG still contains the pre-signing app, so it cannot be
//    reused here. No existing user file is touched: all work stays in tmp.
const staging = mkdtempSync(join(tmpdir(), "afm-adhoc-dmg-"));
try {
  const stagedApp = join(staging, basename(appPath));
  run("cp", ["-R", appPath, stagedApp]);
  copyFileSync(helpPath, join(staging, "打开帮助.txt"));
  // Standard drag-to-Applications affordance, same as the Tauri-generated DMG.
  execFileSync("ln", ["-s", "/Applications", join(staging, "Applications")]);
  run("hdiutil", [
    "create",
    "-volname",
    "A File Manager",
    "-srcfolder",
    staging,
    "-ov",
    "-format",
    "UDZO",
    // Use explicit -o: the positional-output form silently produced a
    // transient image on this runner during local rehearsal.
    "-o",
    dmgPath,
  ]);
  run("hdiutil", ["verify", dmgPath]);
} finally {
  rmSync(staging, { recursive: true, force: true });
}

console.log(
  `ad-hoc macOS fallback ready: ${basename(appPath)} + ${basename(dmgPath)} (打开帮助.txt included)`,
);
