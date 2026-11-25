// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Unified error-localization contract gate (ADR 0025 hard-pin family, like
 * check-ipc-parity). Two deterministic invariants that the Step-3 contract and
 * the F4/F5/F14-F17/F25 findings exposed:
 *
 *   1. JARGON: no locale VALUE leaks an internal identifier (awam-*, KM_*,
 *      settings.yaml, ADR 00xx, sidecar, %APPDATA%, awam://, /audio/transcriptions)
 *      into user-facing copy — the "首次使用镜头" defect class.
 *   2. CONTRACT: every error code that Rust `classify_error` can emit has a
 *      matching `fileOps.error.<code>` string in ALL 6 locales — otherwise the
 *      frontend falls back to raw English / the key itself (silent i18n leak).
 *
 * Pure reads, offline. Run: node scripts/check-i18n-contract.mjs
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const LOCALES = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];

const stripComments = (t) =>
  t.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");

// ---- 1. jargon scan over every locale value --------------------------------
const JARGON = [
  /awam-knowledge/,
  /awam-file/,
  /awam\.[a-z-]+/,
  /awam:\/\//,
  /KM_[A-Z_]+/,
  /settings\.yaml/,
  /ADR\s*0\d+/,
  /sidecar/,
  /\/audio\/transcriptions/,
];
// NOTE: %APPDATA% / ~/Library are legitimate user-facing PATH hints (where the
// default skills folder lives), not internal jargon, so they are NOT banned.
const jargonHits = [];
for (const f of LOCALES) {
  const json = JSON.parse(readFileSync(join(root, "src/i18n/locales", f + ".json"), "utf8"));
  const walk = (obj, path) => {
    for (const [k, v] of Object.entries(obj)) {
      const p = path ? `${path}.${k}` : k;
      if (typeof v === "string") {
        const hit = JARGON.find((re) => re.test(v));
        if (hit) jargonHits.push(`${f} ${p} ~ ${hit}`);
      } else if (v && typeof v === "object") {
        walk(v, p);
      }
    }
  };
  walk(json, "");
}
assert.ok(
  jargonHits.length === 0,
  `internal jargon leaked into user-facing locale copy:\n  ${jargonHits.join("\n  ")}`,
);

// ---- 2. Rust classify_error codes ⊆ fileOps.error keys (all locales) --------
const mod = stripComments(readFileSync(join(root, "src-tauri/src/file_ops/mod.rs"), "utf8"));
const start = mod.indexOf("fn classify_error");
assert.ok(start >= 0, "could not locate classify_error in file_ops/mod.rs");
// slice the function body by brace matching
let depth = 0;
let i = mod.indexOf("{", start);
const bodyStart = i;
for (; i < mod.length; i++) {
  if (mod[i] === "{") depth++;
  else if (mod[i] === "}") {
    depth--;
    if (depth === 0) break;
  }
}
const body = mod.slice(bodyStart, i);
const codes = new Set();
// Only capture the FIRST tuple element of `Some(("code", ...))` returns: the
// quote must be immediately followed by a comma. This excludes string literals
// inside .contains("...") and params tuples like ("name".to_string(), ...).
for (const m of body.matchAll(/\(\s*"([a-z][a-z0-9_]+)",/g)) codes.add(m[1]);
assert.ok(codes.size >= 8, `classify_error extraction looks broken (${codes.size} codes) — refusing to judge`);

for (const f of LOCALES) {
  const json = JSON.parse(readFileSync(join(root, "src/i18n/locales", f + ".json"), "utf8"));
  const keys = new Set(Object.keys(json?.fileOps?.error ?? {}));
  assert.ok(keys.size > 0, `fileOps.error missing in ${f}`);
  const missing = [...codes].filter((c) => !keys.has(c));
  assert.ok(
    missing.length === 0,
    `fileOps.error missing code(s) [${missing.join(", ")}] in ${f} — Rust would emit a code with no localized string`,
  );
}

console.log(
  `i18n-contract checks OK (${codes.size} error codes localized across ${LOCALES.length} locales; 0 jargon leaks).`,
);
