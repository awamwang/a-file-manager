// SPDX-License-Identifier: GPL-3.0-or-later
/** 7.16: behavior contract for the single frontend fs-path entry point. */
import assert from "node:assert/strict";
import {
  fsPathSeparator,
  isPosixAbsolute,
  joinFsPath,
  splitFsPath,
  toNativeFsPath,
} from "../src/state/fsPath.ts";

// POSIX absolute paths keep "/" and their leading root (7.14 regression).
assert.equal(toNativeFsPath("/Users/demo/Docs/"), "/Users/demo/Docs");
assert.equal(toNativeFsPath("/"), "/");
assert.equal(toNativeFsPath("  /Users/demo  "), "/Users/demo");
assert.equal(joinFsPath("/Users/demo", "Downloads"), "/Users/demo/Downloads");
assert.equal(joinFsPath("/", "Users"), "/Users");
assert.equal(fsPathSeparator("/Users/demo"), "/");
assert.deepEqual(splitFsPath("/Users/demo/Docs"), {
  root: "/",
  segments: ["Users", "demo", "Docs"],
});

// `//` is UNC on Windows, not a POSIX root.
assert.equal(isPosixAbsolute("/Users/demo"), true);
assert.equal(isPosixAbsolute("//server/share"), false);
assert.equal(isPosixAbsolute("C:\\Temp"), false);
assert.equal(toNativeFsPath("//server/share/folder"), "\\\\server\\share\\folder");

// Windows drive semantics are unchanged from the pre-port implementation.
assert.equal(toNativeFsPath("G:/Projects/A/demo"), "G:\\Projects\\A\\demo");
assert.equal(toNativeFsPath("C:\\"), "C:\\");
assert.equal(toNativeFsPath("C:/"), "C:\\");
assert.equal(joinFsPath("C:\\", "Temp"), "C:\\Temp");
assert.equal(joinFsPath("C:\\Temp", "a.txt"), "C:\\Temp\\a.txt");
assert.equal(fsPathSeparator("C:\\Temp"), "\\");
assert.deepEqual(splitFsPath("C:\\Temp\\Docs"), {
  root: "C:\\",
  segments: ["Temp", "Docs"],
});

// URI schemes keep their own separators.
assert.equal(toNativeFsPath("afm://guide"), "afm://guide");
assert.equal(toNativeFsPath("http://example.com/a/"), "http://example.com/a");

// Relative paths still normalize to the Windows separator for comparison keys.
assert.equal(toNativeFsPath("a/b/"), "a\\b");

console.log("fs-path checks OK (POSIX / Windows / UNC / URI).");
