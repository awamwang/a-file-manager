// SPDX-License-Identifier: GPL-3.0-or-later
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

function flatten(obj, prefix = "", out = []) {
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      flatten(value, path, out);
    } else {
      out.push(path);
    }
  }
  return out;
}

const enKeys = flatten(en).sort();
const zhKeys = flatten(zh).sort();
const missingInZh = enKeys.filter((k) => !zhKeys.includes(k));
const extraInZh = zhKeys.filter((k) => !enKeys.includes(k));
if (missingInZh.length || extraInZh.length) {
  console.error("Locale key mismatch en ↔ zh-CN");
  for (const k of missingInZh) console.error(`  missing zh-CN: ${k}`);
  for (const k of extraInZh) console.error(`  extra zh-CN: ${k}`);
  process.exit(1);
}

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/i18n-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);

console.log(`i18n locale parity OK (${enKeys.length} keys × en/zh-CN).`);
