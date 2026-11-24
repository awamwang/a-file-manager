// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/link-paste-name-checks.ts
import assert from "node:assert/strict";
import { linkPasteName } from "../src/state/fileClipboard.ts";

// TR-203: the FE name must mirror shortcut.rs::shortcut_file_name on both hosts.
assert.equal(linkPasteName("Report.pdf", false), "Report.pdf - Shortcut.lnk");
assert.equal(linkPasteName("Report.pdf", true), "Report.pdf - Shortcut.link");
assert.equal(linkPasteName("Photos", true), "Photos - Shortcut.link");
assert.notEqual(
  linkPasteName("Report.pdf", true),
  "Report.pdf - Shortcut.lnk",
  "macOS must never produce a .lnk name",
);
assert.equal(
  linkPasteName("Report.pdf", true).endsWith("Report.pdf - Shortcut.link"),
  true,
);

console.log("link paste name checks OK (TR-203 .lnk / .link split).");
