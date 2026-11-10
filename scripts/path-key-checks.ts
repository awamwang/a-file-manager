// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  normalizePathKey,
  normalizeTreePathKey,
  pathsEqual,
  toNativeFsPath,
} from "../src/state/pathKey.ts";
import { normalizeTreePath } from "../src/state/folderTree.ts";
import {
  normalizePathKey as openStateKey,
  toNativeFsPath as openStateNative,
} from "../src/state/openState/model.ts";

assert.equal(toNativeFsPath("G:/Projects/A/demo"), "G:\\Projects\\A\\demo");
assert.equal(toNativeFsPath("afm://guide"), "afm://guide");
assert.equal(toNativeFsPath("C:\\"), "C:\\");
assert.equal(normalizePathKey("  G:\\Projects\\A\\  "), "g:\\projects\\a");
assert.equal(normalizePathKey("g:/projects/a"), "g:\\projects\\a");
assert.ok(pathsEqual("D:/Work", "d:\\work"));
assert.ok(!pathsEqual("D:/Work", "d:\\other"));

assert.equal(normalizeTreePathKey("C:\\"), "c:");
assert.equal(normalizeTreePathKey("C:\\Temp\\"), "c:\\temp");
assert.equal(normalizeTreePath("C:\\"), "c:");
assert.equal(normalizeTreePath("C:/Temp"), "c:\\temp");

assert.equal(openStateNative("a:/b"), toNativeFsPath("a:/b"));
assert.equal(openStateKey("a:/b"), normalizePathKey("a:/b"));

console.log("path-key-checks: ok");
