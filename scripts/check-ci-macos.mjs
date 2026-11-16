// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 7.25 guard: macOS packages must be buildable on GitHub Actions without any
 * Developer ID secret, while the existing Windows NSIS flow stays untouched.
 * This is a static contract — a real .dmg can only be produced on a macOS
 * runner, never in the sandbox this repo is developed in.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  appZipAssetName,
  dmgAssetName,
  macReleaseMatrix,
  tauriDmgFileName,
} from "./release-assets.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");
/** YAML/JS source with comments removed, so a comment cannot satisfy a guard. */
const code = (source) =>
  source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => line.replace(/(^|\s)#.*$/, "$1").replace(/\/\/.*$/, ""))
    .join("\n");

const ci = code(read(".github/workflows/ci.yml"));
const release = code(read(".github/workflows/release.yml"));
const buildFlavor = code(read("scripts/build-flavor.mjs"));
const assets = code(read("scripts/release-assets.mjs"));
const pkg = JSON.parse(read("package.json"));
const conf = JSON.parse(read("src-tauri/tauri.conf.json"));

// 1. CI is a real two-OS matrix, not a Windows job with a Mac afterthought.
assert.ok(ci.includes("runs-on: ${{ matrix.os }}"), "CI must select runner from matrix");
assert.ok(ci.includes("windows-latest"), "CI must keep the Windows runner");
assert.ok(ci.includes("macos-latest"), "CI must add the macOS runner");
assert.ok(ci.includes("rust_target"), "matrix must pin a Rust target per runner");
assert.ok(
  ci.includes("shared-key: ci-${{ matrix.os }}-${{ matrix.rust_target }}"),
  "Rust cache key must be scoped by OS and target",
);
assert.ok(ci.includes("run: npm test"), "both runners must execute the full contract chain");
assert.ok(
  ci.includes("if: runner.os == 'Windows'"),
  "WebView2/NSIS assertions must be gated to Windows",
);
assert.ok(
  !/shell:\s*(powershell|pwsh)/.test(ci),
  "no PowerShell-only step may remain in the shared matrix",
);

// 2. Release builds Mac bundles on a real Mac, with no signing secrets.
assert.ok(release.includes("build-app-macos:"), "release workflow needs a macOS job");
assert.ok(release.includes("runs-on: macos-latest"), "macOS job must run on a Mac runner");
assert.ok(
  release.includes("mac_matrix") && release.includes("macReleaseMatrix"),
  "mac matrix must be derived from the requested packs",
);
assert.ok(release.includes("--target"), "release must pin the Rust target per matrix row");
assert.ok(
  release.includes("dmgAssetName") && release.includes("appZipAssetName"),
  "release must use canonical macOS asset names",
);
assert.ok(release.includes("ditto -c -k --keepParent"), "app bundle must be zipped with ditto");
assert.ok(release.includes("build-app-macos.result"), "finalize must check the macOS job");
// v0.3.0 第二轮实测：finalize 未 checkout 时 `gh release edit` 报
// "fatal: not a git repository"，所有构建都成功却无法 undraft。
{
  const finalize = release.slice(release.indexOf("  finalize:"));
  assert.ok(
    finalize.includes("actions/checkout@v4"),
    "finalize must check out the repo before `gh release edit`",
  );
}
assert.ok(
  !release.includes("secrets.APPLE_"),
  "no APPLE_* secret may be required — signing/notarization stays opt-in",
);
assert.ok(release.includes(".dmg"), "release notes must mention macOS assets");

// 3. Build entry reuses the existing flavor logic, only the bundle set differs.
assert.ok(buildFlavor.includes("--target"), "build-flavor must forward --target");
assert.ok(buildFlavor.includes('"nsis"'), "Windows NSIS default must stay intact");
assert.ok(buildFlavor.includes('"app,dmg"'), "macOS must produce app and dmg");
assert.ok(assets.includes("export function dmgAssetName"), "asset naming helper must exist");

// 4. Naming contract (source name from Tauri vs canonical GitHub asset name).
assert.equal(tauriDmgFileName("0.3.0", "aarch64"), "A File Manager_0.3.0_aarch64.dmg");
assert.equal(tauriDmgFileName("0.3.0", "x64"), "A File Manager_0.3.0_x64.dmg");
assert.equal(dmgAssetName("0.3.0", "core", "aarch64"), "A.File.Manager_0.3.0_aarch64.dmg");
assert.equal(dmgAssetName("0.3.0", "ai", "x64"), "A.File.Manager_0.3.0_x64-ai.dmg");
assert.equal(appZipAssetName("0.3.0", "core", "x64"), "A.File.Manager_0.3.0_x64.app.zip");
assert.equal(
  macReleaseMatrix(["core", "ai"]).length,
  4,
  "two flavors x two architectures",
);
assert.deepEqual(macReleaseMatrix(["feature-pack"]), [], "feature-pack-only releases skip Mac");

// 5. Local config already declares the Mac bundle; no signing identity yet.
for (const target of ["nsis", "app", "dmg"]) {
  assert.ok(conf.bundle.targets.includes(target), `bundle.targets must include ${target}`);
}
assert.ok(conf.bundle.icon.includes("icons/icon.icns"), "icns must be bundled");
assert.equal(conf.bundle.macOS.minimumSystemVersion, "10.15");
assert.equal(conf.bundle.macOS.signingIdentity, undefined, "signing stays opt-in for now");

// 6. The contract itself runs on both CI runners via the shared npm chain.
assert.ok(pkg.scripts.test.includes("npm run test:macos-bundle"));
assert.ok(pkg.scripts.test.includes("npm run test:ci-macos"));

console.log("CI macOS packaging checks OK (7.25).");
