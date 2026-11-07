// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { dropTransferMode } from "../src/state/dropMode.ts";

assert.equal(
  dropTransferMode(["C:\\Temp\\afm-fixture\\a\\file.txt"], "C:\\Temp\\afm-fixture\\b", {}),
  "move",
);
assert.equal(
  dropTransferMode(["C:\\Temp\\afm-fixture\\a\\file.txt"], "D:\\Temp\\afm-fixture\\b", {}),
  "copy",
);
assert.equal(
  dropTransferMode(["C:\\Temp\\afm-fixture\\a\\file.txt"], "C:\\Temp\\afm-fixture\\b", { ctrl: true }),
  "copy",
);
assert.equal(
  dropTransferMode(["D:\\Temp\\afm-fixture\\a\\file.txt"], "C:\\Temp\\afm-fixture\\b", { shift: true }),
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
assert.equal(
  dropTransferMode(["C:\\Temp\\a\\file.txt"], "C:\\Temp\\b", { alt: true }),
  "link",
);
assert.equal(
  dropTransferMode(["C:\\Temp\\a\\file.txt"], "D:\\Temp\\b", { alt: true, ctrl: true }),
  "link",
);

console.log("drop-mode checks OK (incl. Alt=link).");
