// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 7.16 architecture gate: all frontend producers of *real filesystem paths*
 * must delegate shape detection / normalization / joining to fsPath.ts.
 * A fifth local copy is what produced "Path does not exist: Users\wangmeng".
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const consumers = [
  "src/state/navHistory.ts",
  "src/state/pathKey.ts",
  "src/state/navigatorBrowse.ts",
  "src/state/settingsEffectiveDefaults.ts",
];
const sources = consumers.map((rel) => [rel, read(rel)]);

for (const [rel, src] of sources) {
  assert.ok(
    src.includes('from "./fsPath.ts"'),
    `${rel} must delegate path semantics to fsPath.ts`,
  );
  assert.ok(
    !src.includes('startsWith("/") && !'),
    `${rel} must not hand-roll the POSIX-root shape test`,
  );
  assert.ok(
    !src.includes("function toNativeDisplayPath"),
    `${rel} must not keep a private native-path normalizer`,
  );
}

const fsPath = read("src/state/fsPath.ts");
for (const fn of [
  "export function isPosixAbsolute",
  "export function fsPathSeparator",
  "export function toNativeFsPath",
  "export function joinFsPath",
  "export function splitFsPath",
]) {
  assert.ok(fsPath.includes(fn), `fsPath.ts must export ${fn}`);
}

// Existing consumers / contracts keep their public API (thin wrapper).
const pathKey = read("src/state/pathKey.ts");
assert.ok(pathKey.includes("export function toNativeFsPath"));
assert.ok(read("src/state/openState/model.ts").includes('from "../pathKey.ts"'));
assert.ok(read("src/state/navigatorBrowse.ts").includes('from "./pathKey.ts"'));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/fs-path-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "fs-path-checks failed");

console.log("check-fs-path: ok (single entry point + behavior contract)");
