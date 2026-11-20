// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 7.25 guard: macOS packages build on GitHub Actions; Windows NSIS flow is
 * untouched. 7.26: Developer ID signing + notarization is opt-in behind the
 * mac_signing gate — ad-hoc stays the zero-config default. Static contract:
 * a real .dmg (signed or not) can only be produced on a macOS runner.
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
const helpText = read("src-tauri/resources/macos-open-help.txt");
const finalizeAdhoc = code(read("scripts/finalize-macos-adhoc.mjs"));

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
    /actions\/checkout@v\d+/.test(finalize),
    "finalize must check out the repo before `gh release edit`",
  );
}
// 2b. 7.26 opt-in signing: APPLE_* secrets are allowed, but ONLY behind the
// mac_signing gate. Ad-hoc must remain the zero-config default, and a partial
// credential set must never hard-fail the build.
assert.ok(release.includes("mac_signing:"), "prepare must expose a mac_signing output");
assert.ok(
  release.includes("echo \"mac_signing=${MAC_SIGNING}\""),
  "prepare must publish the derived signing boolean",
);
for (const secret of [
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
]) {
  assert.ok(
    release.includes(`secrets.${secret}`),
    `signing must wire the ${secret} secret`,
  );
}
assert.ok(
  release.includes("if: needs.prepare.outputs.mac_signing == 'true'"),
  "certificate import must be gated on the signing boolean",
);
assert.ok(
  release.includes("security import") && release.includes("-T /usr/bin/codesign"),
  "import step must install the Developer ID for codesign",
);
assert.ok(
  release.includes("security set-key-partition-list"),
  "codesign must be allowed to use the imported key (partition list)",
);
assert.ok(
  release.includes("unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID"),
  "ad-hoc builds must unset empty APPLE_* vars (Tauri treats set-but-empty as configured)",
);
assert.ok(
  release.includes('if [ "${MAC_SIGNING}" != "true" ]'),
  "ad-hoc fallback must be the default when signing is off",
);
assert.ok(
  release.includes("直接双击打开") && release.includes("右键 → 打开只在部分 macOS 版本有效"),
  "release notes must describe both signed and ad-hoc first-launch behavior",
);
assert.ok(
  release.includes("xattr -dr com.apple.quarantine") && release.includes("仍要打开"),
  "unsigned release notes must give a fallback that works when right-click Open fails",
);
assert.ok(release.includes(".dmg"), "release notes must mention macOS assets");

// 2d. Feed sidecars must also be built on all three hosts, then merged into a
// single manifest by ONE job — concurrent `gh release upload ... manifest.json`
// would clobber each other and leave mac users with no matching artifact.
const feedJob = release.slice(
  release.indexOf("  build-pack-feed:"),
  release.indexOf("  merge-pack-feed:"),
);
const mergeJob = release.slice(
  release.indexOf("  merge-pack-feed:"),
  release.indexOf("  finalize:"),
);
for (const label of ["windows-latest", "macos-latest", "macos-15-intel"]) {
  assert.ok(feedJob.includes(label), `feed build matrix must include ${label}`);
}
for (const key of ["windows-x86_64", "macos-aarch64", "macos-x86_64"]) {
  assert.ok(feedJob.includes(key), `feed build matrix must declare platform ${key}`);
}
assert.ok(feedJob.includes("--platform"), "feed build must tag every entry with its platform");
assert.ok(
  feedJob.includes('name_suffix: "-x64"'),
  "Mac x64 sidecars need a -x64 suffix so flat Release assets never collide",
);
assert.ok(
  feedJob.includes("actions/upload-artifact@v4"),
  "each host's feed slice must be handed off as an Actions artifact",
);
assert.ok(
  !/gh release upload/.test(feedJob),
  "feed build jobs must not upload to Release directly (would clobber manifest.json)",
);
assert.ok(
  mergeJob.includes("actions/download-artifact@v4") && mergeJob.includes("merge-feed"),
  "merge job must download all slices and fold them into one feed",
);
assert.ok(
  mergeJob.includes("gh release upload") && mergeJob.includes("GH_FEED_BASE"),
  "merge job owns the single flat-URL Release upload",
);
assert.ok(
  release.includes("merge-pack-feed.result") &&
    release.includes("feed_build") &&
    release.includes("feed_merge"),
  "finalize must check both feed stages (a skipped merge after a failed build is not success)",
);
const feedBuilder = read("scripts/build_pack_feed.py");
assert.ok(
  feedBuilder.includes('add_parser("merge-feed"'),
  "feed builder must expose the merge-feed subcommand",
);
// macOS sidecars follow the same zero-credential ad-hoc policy as AFM.app:
// the build script signs onefiles on darwin hosts; no Developer ID / notarization.
assert.ok(
  feedBuilder.includes("def adhoc_sign_macos") &&
    feedBuilder.includes('codesign", "--force", "--sign", "-", "--timestamp=none"') &&
    feedBuilder.includes("Signature=adhoc") &&
    feedBuilder.includes('platform_key.startswith("macos-")'),
  "feed builder must ad-hoc sign macOS onefiles (no Developer ID / notarization)",
);
assert.ok(
  !/APPLE_(CERTIFICATE|ID|PASSWORD|TEAM_ID)/.test(feedJob) &&
    !/Developer ID|notarytool|notariz/.test(feedJob),
  "feed sidecar builds must stay on the zero-credential ad-hoc path",
);

// 2c. 7.27: the zero-credential path must not ship a completely unsigned app,
// and the help file must exist both in the bundle and at DMG/Release level.
assert.ok(
  release.includes("Apple signing credentials incomplete or absent — macOS builds use ad-hoc fallback"),
  "prepare must log the automatic no-certificate fallback",
);
assert.ok(
  release.includes("finalize-macos-adhoc.mjs") &&
    release.includes("if: needs.prepare.outputs.mac_signing != 'true'"),
  "unsigned builds must run the ad-hoc + in-package-help finalizer",
);
assert.ok(
  release.includes("A.File.Manager_${VERSION}_macos-open-help.txt") &&
    release.includes("macos-open-help.txt"),
  "help file must be published as a canonical release asset",
);
assert.ok(
  conf.bundle.resources.includes("resources/macos-open-help.txt"),
  "bundle.resources must embed the help file so signed builds ship it too",
);
assert.ok(
  finalizeAdhoc.includes('codesign", ["--force", "--deep", "--sign", "-", "--timestamp=none", appPath]') &&
    finalizeAdhoc.includes('Signature=adhoc') &&
    finalizeAdhoc.includes('run("codesign", ["--verify", "--deep", "--strict"'),
  "finalizer must create and verify an explicit ad-hoc signature",
);
assert.ok(
    finalizeAdhoc.includes('copyFileSync(helpPath, join(staging, "打开帮助.txt"))') &&
    finalizeAdhoc.includes('"-volname"') &&
    finalizeAdhoc.includes('"-o"') &&
    finalizeAdhoc.includes('run("hdiutil", ["verify", dmgPath])'),
  "finalizer must rebuild a DMG with root-level help and verify the image",
);
assert.ok(
  finalizeAdhoc.includes("Contents\", \"Resources\", \"resources\", \"macos-open-help.txt") &&
    finalizeAdhoc.includes("请检查 tauri.conf.json 的 bundle.resources"),
  "finalizer must fail fast if the app bundle lost its embedded help",
);
for (const phrase of [
  "系统设置",
  "隐私与安全性",
  "仍要打开",
  "xattr -dr com.apple.quarantine",
  "不要点“移到废纸篓”",
  "右键 → 打开",
  "Developer ID",
]) {
  assert.ok(helpText.includes(phrase), `macOS help text must explain: ${phrase}`);
}

// 3. Build entry reuses the existing flavor logic, only the bundle set differs.
assert.ok(buildFlavor.includes("--target"), "build-flavor must forward --target");
assert.ok(buildFlavor.includes('"nsis"'), "Windows NSIS default must stay intact");
assert.ok(buildFlavor.includes('"app,dmg"'), "macOS must produce app and dmg");
assert.ok(assets.includes("export function dmgAssetName"), "asset naming helper must exist");

// 4. Naming contract (source name from Tauri vs canonical GitHub asset name).
assert.equal(tauriDmgFileName("0.3.0", "aarch64"), "A File Manager_0.3.0_aarch64.dmg");
assert.equal(tauriDmgFileName("0.3.0", "x64"), "A File Manager_0.3.0_x64.dmg");
assert.equal(dmgAssetName("0.3.0", "core", "aarch64"), "A.File.Manager_0.3.0_aarch64-lite.dmg");
assert.equal(dmgAssetName("0.3.0", "ai", "x64"), "A.File.Manager_0.3.0_x64.dmg");
assert.equal(appZipAssetName("0.3.0", "core", "x64"), "A.File.Manager_0.3.0_x64-lite.app.zip");
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

console.log("CI macOS packaging checks OK (7.25 + 7.26 signing + 7.27 help + 3-host feed).");
