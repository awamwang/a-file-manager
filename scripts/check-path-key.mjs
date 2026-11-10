// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");
const registerUrl = pathToFileURL(
  join(root, "scripts/register-ts-resolve.mjs"),
).href;

const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/path-key-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "path-key-checks failed");

const pathKey = read("src/state/pathKey.ts");
const browse = read("src/state/navigatorBrowse.ts");
const tree = read("src/state/folderTree.ts");
const model = read("src/state/openState/model.ts");
const rustPaths = read("src-tauri/src/paths.rs");
const wgPaths = read("src-tauri/src/work_groups/paths.rs");
const lib = read("src-tauri/src/lib.rs");

assert.ok(pathKey.includes("export function toNativeFsPath"));
assert.ok(pathKey.includes("export function normalizePathKey"));
assert.ok(pathKey.includes("export function normalizeTreePathKey"));
assert.ok(model.includes('from "../pathKey"'));
assert.ok(browse.includes('from "./pathKey"'));
assert.ok(tree.includes("normalizeTreePathKey"));
assert.ok(lib.includes("mod paths;"));
assert.ok(rustPaths.includes("pub fn to_native_fs_path"));
assert.ok(wgPaths.includes("pub use crate::paths::"));

console.log("check-path-key: ok");
