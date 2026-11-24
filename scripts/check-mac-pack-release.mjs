// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * ADR 0022 / 2026-11-24：三个 Feature Pack 的 macOS 发布链路门禁。
 *
 * 防止「CI 注释宣称 Mac 不可行 → 平台 slice 容错 → manifest 静默缺 Mac 条目」
 * 这一类纸面接通、实际空壳的回归。只做静态断言（配置 + 构建器能力），
 * 不跑网络、不做 PyInstaller 冻结；真实冻结由 CI 的三平台矩阵承担。
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const pkg = JSON.parse(read("package.json"));
const release = read(".github/workflows/release.yml");
const builder = read("scripts/build_pack_feed.py");

// 1. Gate itself must be wired — an orphan gate protects nothing (AGENTS.md / H8).
assert.equal(pkg.scripts["test:mac-pack-release"], "node scripts/check-mac-pack-release.mjs");
assert.ok(
  pkg.scripts.test.includes("npm run test:mac-pack-release"),
  "test:mac-pack-release must be part of the aggregated npm test chain",
);

// 2. Job slices. Slice by job id so a needle in an unrelated job cannot satisfy us.
const between = (from, to) => {
  const a = release.indexOf(from);
  const b = release.indexOf(to);
  assert.ok(a >= 0 && b > a, `cannot locate ${from} / ${to} in release.yml`);
  return release.slice(a, b);
};
const sharedJob = between("  build-pack-feed:\n", "  build-pack-feed-intel:");
const intelJob = between("  build-pack-feed-intel:\n", "  merge-pack-feed:");
const mergeJob = between("  merge-pack-feed:\n", "  finalize:");

assert.ok(sharedJob.includes("os: macos-latest"), "Apple Silicon runner present");
assert.ok(sharedJob.includes("platform: macos-aarch64"), "aarch64 feed key present");
assert.ok(intelJob.includes("os: macos-15-intel"), "Intel slice uses the public macOS runner");
assert.ok(intelJob.includes("platform: macos-x86_64"), "Intel feed key present");
assert.ok(intelJob.includes('name_suffix: "-x64"'), "Intel artifact name must not collide with arm64");
assert.ok(!release.includes("enable_mac_intel"), "Intel slice must not be opt-in / skippable");

// 3. Fail loud: a release with a missing platform is worse than no release.
// Match config lines only — prose in the job comments explains the removed
// setting and must not trip the gate (AGENTS.md: gates fail on behavior, not words).
const hasKey = (job, key) => new RegExp(`^\\s*${key}\\s*:`, "m").test(job);
assert.ok(
  !hasKey(sharedJob, "continue-on-error") && !hasKey(intelJob, "continue-on-error"),
  "pack feed builds must block the release when any platform fails",
);
assert.ok(
  !/^\s*if:\s*always\(\)/m.test(mergeJob),
  "merge must not publish a partial manifest",
);
assert.ok(
  mergeJob.includes("if: needs.prepare.outputs.want_feature_pack == 'true'"),
  "merge relies on the default all-needs-succeeded rule",
);

// 4. Every platform slice builds all three packs — one missing pack is a silent hole.
for (const [label, job] of [["shared", sharedJob], ["intel", intelJob]]) {
  for (const id of ["awam.tool-runtime", "awam.file-gateway", "awam.knowledge"]) {
    assert.ok(job.includes(`--pack-id ${id}`), `${label} slice is missing ${id}`);
  }
}

// 5. Builder contract: platform-keyed entries + ad-hoc signature on macOS.
assert.ok(builder.includes("def artifact_platform_key("), "builder normalizes host platform keys");
assert.ok(builder.includes('os_key = "macos"'), "builder recognizes macOS hosts");
assert.ok(builder.includes("def adhoc_sign_macos("), "Mac artifacts must be ad-hoc signed");
assert.ok(
  builder.includes('if platform_key.startswith("macos-")'),
  "signing must be keyed to Mac entries",
);
assert.ok(
  builder.includes('(e.get("packId"), e.get("platform") or "")'),
  "feed merge dedups by (packId, platform), never by packId alone",
);

console.log("mac-pack-release OK (3 packs × Windows / Apple Silicon / Intel; no partial manifest).");
