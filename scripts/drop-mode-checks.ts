// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { dropTransferMode } from "../src/state/dropMode.ts";

assert.equal(
  dropTransferMode(["C:\\a\\file.txt"], "C:\\b", {}),
  "move",
);
assert.equal(
  dropTransferMode(["C:\\a\\file.txt"], "D:\\b", {}),
  "copy",
);
assert.equal(
  dropTransferMode(["C:\\a\\file.txt"], "C:\\b", { ctrl: true }),
  "copy",
);
assert.equal(
  dropTransferMode(["D:\\a\\file.txt"], "C:\\b", { shift: true }),
  "move",
);
assert.equal(
  dropTransferMode(["\\\\server\\share\\a.txt"], "\\\\server\\other", {}),
  "move",
);
assert.equal(
  dropTransferMode(["\\\\server\\share\\a.txt"], "\\\\other\\share", {}),
  "copy",
);

console.log("drop-mode checks OK.");
