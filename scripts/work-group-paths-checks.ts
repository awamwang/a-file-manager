// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the "Add to Work Group" folder-path reduction.
// Pinned by scripts/check-work-group-paths.mjs. Guards the W2 external chain:
// selecting folders in the navigator and adding them to a Work Group must keep only
// directories, expand a multi-select, and fall back to the right-clicked entry.
import assert from "node:assert/strict";
import { folderPathsForWorkGroup } from "../src/components/navigator/workGroupPaths.ts";

const dir = (name: string) => ({ name, path: `C:/afm/${name}`, kind: "dir" as const, size: 0 });
const file = (name: string) => ({ name, path: `C:/afm/${name}`, kind: "file" as const, size: 1 });

const a = dir("a");
const b = dir("b");
const f = file("f.txt");

// A file contributes nothing — files are never added to a Work Group.
assert.deepEqual(folderPathsForWorkGroup(f, [f.path], [f]), []);

// Single selection (dir not part of a >1 multi-select) → just that dir, even if empty selection.
assert.deepEqual(folderPathsForWorkGroup(a, [a.path], [a, b, f]), [a.path]);
assert.deepEqual(folderPathsForWorkGroup(a, [], [a, b]), [a.path]);

// Multi-select → every selected dir, files skipped, listing order preserved.
assert.deepEqual(folderPathsForWorkGroup(a, [a.path, f.path, b.path], [a, f, b]), [a.path, b.path]);

// Fallback: entry is part of a >1 multi-select but the listing no longer contains any
// selected dir → keep the entry itself rather than returning nothing.
assert.deepEqual(folderPathsForWorkGroup(a, [a.path, b.path], [f]), [a.path]);

console.log("work-group-paths OK (files skipped; multi-select reduces to selected dirs; entry fallback).");
