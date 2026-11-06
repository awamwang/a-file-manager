// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { formatBytes, selectionByteSize } from "../src/state/formatBytes.ts";

assert.equal(formatBytes(512), "512 B");
assert.equal(formatBytes(2048), "2.0 KB");
assert.equal(
  selectionByteSize(
    ["a", "b"],
    [
      { path: "a", kind: "file", size: 10 },
      { path: "b", kind: "dir", size: 999 },
      { path: "c", kind: "file", size: 50 },
    ],
  ),
  10,
);

console.log("status-extras checks OK (AIA-195).");
