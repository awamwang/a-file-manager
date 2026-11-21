// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Full-surface FE↔BE invoke-name parity gate (ADR 0025 hard-pin rule, generalized).
 *
 * A Tauri command renamed or removed on the backend while the frontend still invokes the old
 * name fails silently at runtime — no compile error, no test red. The per-domain gates pin
 * their slices; this gate pins the WHOLE IPC surface:
 *   1. registered = every leaf command name inside lib.rs `generate_handler![ … ]`;
 *   2. frontend = every snake_case string literal passed to `invoke(...)` under src/;
 *   3. every frontend literal must resolve to a registered command.
 * Plugin-style calls (`plugin:xxx|yyy`) and non-literal names are out of scope by design.
 * Pure reads, offline. Run with AFM_PARITY_REPORT=1 to print offenders instead of failing
 * (tuning aid only — CI always asserts).
 */
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const reportOnly = process.env.AFM_PARITY_REPORT === "1";

const stripComments = (t) =>
  t.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");

// --- registered commands: leaf names inside every generate_handler![ … ] in lib.rs ------------
const lib = stripComments(readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8"));
const registered = new Set();
for (const m of lib.matchAll(/generate_handler!\[/g)) {
  let depth = 1;
  let i = m.index + m[0].length;
  while (i < lib.length && depth > 0) {
    const c = lib[i];
    if (c === "[") depth += 1;
    else if (c === "]") depth -= 1;
    i += 1;
  }
  const block = lib.slice(m.index + m[0].length, i - 1);
  for (const tok of block.matchAll(/([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)/g)) {
    registered.add(tok[1].split("::").pop());
  }
}
assert.ok(registered.size >= 80, `invoke_handler extraction looks broken (${registered.size} commands) — refusing to judge parity against a stub set`);

// --- frontend invoke literals ----------------------------------------------------------------
const SRC = join(root, "src");
const INVOKE = /\binvoke\s*(?:<[^>()]*>)?\s*\(\s*["'`]([^"'`]+)["'`]/g;
const offenders = [];
let feCount = 0;
function walk(dir) {
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name);
    const st = statSync(abs);
    if (st.isDirectory()) { walk(abs); continue; }
    if (!/\.(ts|vue)$/.test(name)) continue;
    const text = stripComments(readFileSync(abs, "utf8"));
    for (const m of text.matchAll(INVOKE)) {
      const name = m[1];
      if (!/^[a-z][a-z0-9_]*$/.test(name)) continue; // plugins, dynamic names, events: out of scope
      feCount += 1;
      if (!registered.has(name)) offenders.push(`${relative(root, abs)} -> ${name}`);
    }
  }
}
walk(SRC);
assert.ok(feCount >= 50, `frontend scan found only ${feCount} invoke literals — scan regex likely broken`);

const msg =
  `frontend invokes ${offenders.length} command name(s) not registered in lib.rs invoke_handler:\n  ` +
  offenders.join("\n  ");
if (offenders.length) {
  if (reportOnly) console.log(`REPORT MODE — ${msg}`);
  else assert.fail(msg);
}

console.log(
  `ipc-parity checks OK (${feCount} FE invoke literals all resolve to ${registered.size} registered commands).`,
);
