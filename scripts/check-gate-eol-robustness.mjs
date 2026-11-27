// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Gate EOL-robustness guard (anti-recurrence).
 *
 * Why this exists:
 *  - The repository stores source blobs with CRLF, and GitHub's Windows runners
 *    check the tree out with CRLF (core.autocrlf=true is the default there).
 *  - A check script that asserts a *multi-line needle* (a string / template /
 *    regex literal containing an escaped \n) against content read from disk is
 *    therefore platform-dependent: `haystack.includes("line1\nline2")` silently
 *    fails when the on-disk file uses `line1\r\nline2`. This bit
 *    check-navigator-drill.mjs (the `MAX_VISITED\n … .max(…)` assertion against
 *    drill.rs) and is a latent trap in every gate that does the same.
 *
 * The durable fix lives at the assertion layer, NOT the checkout layer:
 * forcing `eol=lf` in .gitattributes cannot rewrite the already-committed CRLF
 * blobs without a repo-wide `--renormalize` (a conflict-heavy diff on a shared
 * repo). Normalizing at read time is correct regardless of how a file is stored
 * or checked out, so every check script that reads files and uses a multi-line
 * needle MUST fold CRLF away first: `.replace(/\r\n/g, "\n")`.
 *
 * This gate fails `npm test` if any scripts/check-*.mjs reintroduces that
 * pattern without normalizing, so the CI red-on-Windows class cannot return.
 */
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8").replace(/\r\n/g, "\n");

const scriptsDir = join(root, "scripts");
const files = readdirSync(scriptsDir)
  .filter((name) => name.startsWith("check-") && name.endsWith(".mjs"))
  .sort();

// Reads content from disk (directly or through a local read/readFrom helper).
const READS_DISK = /readFileSync\s*\(/;

// An assertion whose *needle spans lines*: a quoted / template string literal
// that contains an escaped \n and is passed to a containment/match call. A real
// newline in the needle would make it a template literal spanning physical
// lines, which the same regex catches via [\s\S].
const MULTILINE_NEEDLE = [
  // .includes("a\nb") / .includes(`a\nb`) / .includes('a\nb')
  /\.includes\(\s*(["'`])(?:\\[\s\S]|(?:(?!\1)[\s\S]))*?\\n(?:\\[\s\S]|(?:(?!\1)[\s\S]))*?\1/,
  // .match(/a\nb/) / .test(/a\nb/) / .replace(/a\nb/, …) — regex literal with \n
  /\.(?:match|test|replace)\(\s*\/(?:\\[\s\S]|\[(?:\\[\s\S]|[^\]])*\]|[^\/\s\S])+\\n(?:\\[\s\S]|\[(?:\\[\s\S]|[^\]])*\]|[^\/\s\S])+\/[dgimsuvy]*/,
];

// Folds CRLF away so multi-line needles are EOL-agnostic.
const NORMALIZES = /replace\(\s*\/\\r\\n\s*\/[gimsuvy]*\s*,\s*(["'`])\\n\1/;

const offenders = [];
for (const name of files) {
  const src = read(join("scripts", name));
  const readsDisk = READS_DISK.test(src);
  const hasMultiLineNeedle = MULTILINE_NEEDLE.some((re) => re.test(src));
  const normalizes = NORMALIZES.test(src);
  if (readsDisk && hasMultiLineNeedle && !normalizes) {
    offenders.push(name);
  }
}

assert.equal(
  offenders.length,
  0,
  "Gate scripts must be EOL-agnostic. These read source files and assert a " +
    "multi-line needle without normalizing CRLF, so they give false negatives " +
    "on a Windows (CRLF) checkout. Fold CRLF in the read(), e.g. " +
    '`.replace(/\\r\\n/g, "\\n")`:\n  ' +
    offenders.join("\n  "),
);

console.log(
  `gate-eol-robustness checks OK (${files.length} check scripts are EOL-agnostic).`,
);
