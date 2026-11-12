// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * FIL-63：多包发版脚本 / CI / 命名约定门禁（不跑真实 NSIS）。
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  ALL_PACK_KINDS,
  featurePackAssetName,
  listFeaturePacks,
  nsisAssetName,
  parsePackKinds,
  PRODUCT_ASSET_PREFIX,
  tauriNsisFileName,
} from "./release-assets.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const pkg = JSON.parse(read("package.json"));
const releaseYml = read(".github/workflows/release.yml");
const prepare = read("scripts/prepare-release.mjs");
const buildFlavor = read("scripts/build-flavor.mjs");
const buildArts = read("scripts/build-release-artifacts.mjs");
const skillPath = join(root, ".cursor/skills/afm-multi-pack-release/SKILL.md");

assert.equal(pkg.scripts["prepare-release"], "node scripts/prepare-release.mjs");
assert.equal(pkg.scripts["package:feature-packs"], "node scripts/package-feature-packs.mjs");
assert.equal(pkg.scripts["build:release-artifacts"], "node scripts/build-release-artifacts.mjs");
assert.equal(pkg.scripts["test:multi-pack-release"], "node scripts/check-multi-pack-release.mjs");

assert.ok(buildFlavor.includes("--bundle"), "build-flavor 支持 --bundle");
assert.ok(prepare.includes("--dry-run"), "prepare-release 支持 --dry-run");
assert.ok(buildArts.includes("--dry-run"), "build-release-artifacts 支持 --dry-run");

assert.ok(releaseYml.includes("flavor_matrix"), "release.yml 输出 flavor matrix");
assert.ok(releaseYml.includes("feature-pack"), "release.yml 包含 feature-pack");
assert.ok(releaseYml.includes("gh release upload"), "release.yml 上传资产");
assert.ok(releaseYml.includes("x64-ai-setup.exe"), "release notes 提及 AI 资产名");
assert.ok(!releaseYml.includes("tauri-apps/tauri-action"), "多包发版不再依赖单包 tauri-action");

assert.equal(nsisAssetName("0.3.0", "core"), `${PRODUCT_ASSET_PREFIX}_0.3.0_x64-setup.exe`);
assert.equal(nsisAssetName("0.3.0", "ai"), `${PRODUCT_ASSET_PREFIX}_0.3.0_x64-ai-setup.exe`);
assert.equal(
  featurePackAssetName("0.3.0", "tool-runtime"),
  `${PRODUCT_ASSET_PREFIX}_0.3.0_feature-pack-tool-runtime.zip`,
);
assert.equal(tauriNsisFileName("0.3.0"), "A File Manager_0.3.0_x64-setup.exe");
assert.deepEqual(parsePackKinds("all"), [...ALL_PACK_KINDS]);
assert.deepEqual(parsePackKinds("core,feature-pack"), ["core", "feature-pack"]);

const packs = listFeaturePacks();
assert.ok(
  packs.some((p) => p.dirName === "tool-runtime"),
  "应能发现 packs/tool-runtime",
);
assert.ok(!packs.some((p) => p.dirName === "_template"), "排除 _template");

assert.ok(existsSync(skillPath), "仓库内 Agent skill 存在");
const skill = read(".cursor/skills/afm-multi-pack-release/SKILL.md");
assert.ok(skill.includes("prepare-release"), "skill 说明 prepare-release");
assert.ok(skill.includes("dry-run") || skill.includes("--dry-run"), "skill 说明 dry-run");
assert.ok(skill.includes("GITHUB_TOKEN") || skill.includes("GH_TOKEN"), "skill 说明 token");

console.log("multi-pack-release OK (FIL-63: core / ai / feature-pack @ one semver).");
