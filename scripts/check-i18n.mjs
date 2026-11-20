// SPDX-License-Identifier: GPL-3.0-or-later
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const localeIds = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
const catalogs = Object.fromEntries(
  localeIds.map((id) => [
    id,
    JSON.parse(readFileSync(join(root, `src/i18n/locales/${id}.json`), "utf8")),
  ]),
);

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

const keySets = Object.fromEntries(
  localeIds.map((id) => [id, flatten(catalogs[id]).sort()]),
);
const enKeys = keySets.en;

/** A registry key missing from any catalog would render as a raw key at runtime. */
let failed = false;
const registrySource = readFileSync(join(root, "src/help/registry.ts"), "utf8");
const registryKeys = [
  ...new Set(
    [...registrySource.matchAll(/\b(?:titleKey|hintKey|bodyKey):\s*"([^"]+)"/g)].map(
      (match) => match[1],
    ),
  ),
].sort();
for (const key of registryKeys) {
  for (const id of localeIds) {
    if (!keySets[id].includes(key)) {
      failed = true;
      console.error(`Guidance registry key missing in ${id}: ${key}`);
    }
  }
}
if (!registryKeys.length) {
  failed = true;
  console.error("Guidance registry keys could not be extracted from src/help/registry.ts");
}

for (const id of localeIds) {
  if (id === "en") continue;
  const keys = keySets[id];
  const missing = enKeys.filter((k) => !keys.includes(k));
  const extra = keys.filter((k) => !enKeys.includes(k));
  if (missing.length || extra.length) {
    failed = true;
    console.error(`Locale key mismatch en ↔ ${id}`);
    for (const k of missing) console.error(`  missing ${id}: ${k}`);
    for (const k of extra) console.error(`  extra ${id}: ${k}`);
  }
}
if (failed) process.exit(1);

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/i18n-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);

console.log(
  `i18n locale parity OK (${enKeys.length} keys × ${localeIds.join("/")}; ${registryKeys.length} guidance registry keys).`,
);
