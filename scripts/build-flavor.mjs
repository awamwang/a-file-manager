// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-222 dual packaging entry.
 *
 * Usage:
 *   node scripts/build-flavor.mjs ai|core [--dev]
 *   node scripts/build-flavor.mjs ai|core --bundle [--target <rust-triple>]
 *
 * - ai:   VITE_AFM_AI_ADDON=1 + Cargo default features (ai-addon)
 * - core: VITE_AFM_AI_ADDON=0 + Cargo --no-default-features
 *
 * With --dev: runs `tauri dev`.
 * With --bundle: runs full `tauri build`（前端由 beforeBuildCommand 构建）。
 *   Bundle targets follow the host: nsis on Windows, app+dmg on macOS
 *   (7.25 GitHub CI packaging). `--target` is forwarded to cargo/tauri, and
 *   artifacts then live under `target/<triple>/release/bundle/`.
 * Otherwise: `vue-tsc` + `vite build` then prints matching `tauri build` cargo args.
 */
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const flavor = (process.argv[2] || "").toLowerCase();
const isDev = process.argv.includes("--dev");
const doBundle = process.argv.includes("--bundle");
const targetIndex = process.argv.indexOf("--target");
const target = targetIndex >= 0 ? process.argv[targetIndex + 1] : "";

if (flavor !== "ai" && flavor !== "core") {
  console.error("Usage: node scripts/build-flavor.mjs ai|core [--dev|--bundle] [--target <triple>]");
  process.exit(1);
}
if (targetIndex >= 0 && (!target || target.startsWith("--"))) {
  console.error("--target needs a Rust target triple");
  process.exit(1);
}

const aiOn = flavor === "ai";
const env = {
  ...process.env,
  VITE_AFM_AI_ADDON: aiOn ? "1" : "0",
};

function run(cmd, args) {
  console.log(`> ${cmd} ${args.join(" ")}`);
  const r = spawnSync(cmd, args, { cwd: root, env, stdio: "inherit", shell: true });
  if (r.status !== 0) process.exit(r.status ?? 1);
}

if (isDev) {
  const cargoArgs = aiOn ? [] : ["--no-default-features"];
  // tauri forwards after `--` to cargo.
  run("npm", ["run", "tauri", "--", "dev", ...(cargoArgs.length ? ["--", ...cargoArgs] : [])]);
  process.exit(0);
}

if (doBundle) {
  // beforeBuildCommand 会跑 npm run build；环境变量 VITE_AFM_AI_ADDON 传入 vite。
  // Windows 仍显式 nsis（FIL-63 契约）；macOS 出 app + dmg（7.25 CI 打包）。
  const bundles =
    process.platform === "win32" ? "nsis" : process.platform === "darwin" ? "app,dmg" : "appimage";
  const args = ["run", "tauri", "--", "build", "--bundles", bundles];
  if (target) args.push("--target", target);
  if (!aiOn) args.push("--", "--no-default-features");
  run("npm", args);
  console.log(
    `\n${bundles} bundle flavor=${flavor} target=${target || "host"} (VITE_AFM_AI_ADDON=${env.VITE_AFM_AI_ADDON}) done.`,
  );
  process.exit(0);
}

run("npx", ["vue-tsc", "--noEmit"]);
run("npx", ["vite", "build"]);

const cargoHint = aiOn
  ? "cargo build --manifest-path src-tauri/Cargo.toml --release"
  : "cargo build --manifest-path src-tauri/Cargo.toml --release --no-default-features";

console.log(`\nFrontend flavor=${flavor} (VITE_AFM_AI_ADDON=${env.VITE_AFM_AI_ADDON}) built.`);
console.log(`Rust: ${cargoHint}`);
const hintBundles = process.platform === "win32" ? "nsis" : "app,dmg";
console.log(
  aiOn
    ? `Tauri package (AI): npm run tauri -- build --bundles ${hintBundles}`
    : `Tauri package (Core): npm run tauri -- build --bundles ${hintBundles} -- --no-default-features`,
);
console.log("或: node scripts/build-flavor.mjs " + flavor + " --bundle");
