// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Render the Homebrew Cask for A File Manager from canonical macOS release assets.
 *
 * The tap is self-hosted (awamwang/homebrew-tap) because the build is ad-hoc
 * signed: official homebrew/cask no longer accepts casks that fail Gatekeeper.
 * We keep a declarative postflight_steps xattr step instead of a legacy Ruby
 * `postflight` block (deprecated in Homebrew 7, removed 2027-12-11).
 *
 * Usage:
 *   node scripts/sync-homebrew-tap.mjs \
 *     --version 0.4.0 \
 *     --aarch64 dist/release/A.File.Manager_0.4.0_aarch64.app.zip \
 *     --x64 dist/release/A.File.Manager_0.4.0_x64.app.zip \
 *     --out dist/release/a-file-manager.rb
 *
 * Checksums can also be passed directly from the GitHub asset `digest` field:
 *   --sha-aarch64 <hex> --sha-x64 <hex>
 */
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const templatePath = join(root, "packaging/homebrew-tap/a-file-manager.rb.tmpl");

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    assert.ok(flag.startsWith("--"), `意外的参数: ${flag}`);
    const value = argv[i + 1];
    assert.ok(value !== undefined && !value.startsWith("--"), `缺少参数值: ${flag}`);
    out[flag.slice(2)] = value;
    i += 1;
  }
  return out;
}

const opts = parseArgs(process.argv.slice(2));
assert.match(opts.version ?? "", /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/, `--version 非法: ${opts.version}`);
assert.ok(opts.out, "缺少 --out 输出路径");

function shaFor(arch) {
  const direct = opts[`sha-${arch}`];
  if (direct) {
    assert.match(direct, /^[0-9a-f]{64}$/, `--sha-${arch} 必须是 64 位小写 hex`);
    return direct;
  }
  const asset = opts[arch];
  assert.ok(asset, `缺少 --${arch} 资产路径或 --sha-${arch}`);
  assert.ok(existsSync(asset), `资产不存在: ${asset}`);
  return createHash("sha256").update(readFileSync(asset)).digest("hex");
}

const shaAarch64 = shaFor("aarch64");
const shaX64 = shaFor("x64");
const template = readFileSync(templatePath, "utf8");
const rendered = template
  .replaceAll("__VERSION__", opts.version)
  .replaceAll("__SHA256_AARCH64__", shaAarch64)
  .replaceAll("__SHA256_X64__", shaX64);
assert.ok(!rendered.includes("__"), "Cask 模板仍有未替换占位符");

mkdirSync(dirname(opts.out), { recursive: true });
writeFileSync(opts.out, rendered, "utf8");
console.log(
  `Cask rendered: ${opts.out} (v${opts.version}; ` +
    `aarch64 ${shaAarch64.slice(0, 12)}… / x64 ${shaX64.slice(0, 12)}…)`,
);
