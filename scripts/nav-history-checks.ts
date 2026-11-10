// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  beginHistoryNavigation,
  buildBreadcrumbs,
  buildNavigatorBreadcrumbs,
  canGoBack,
  canGoForward,
  endHistoryNavigation,
  navigableParentPath,
  popBack,
  popForward,
  recordNavHistory,
  resetNavHistory,
} from "../src/state/navHistory.ts";
import { GUIDE_PATH } from "../src/state/guide/path.ts";

resetNavHistory();
assert.equal(canGoBack.value, false);
assert.equal(canGoForward.value, false);

recordNavHistory("C:\\Temp\\afm-fixture", "C:\\Temp\\afm-fixture\\Docs");
assert.equal(canGoBack.value, true);
assert.equal(canGoForward.value, false);

const back = popBack("C:\\Temp\\afm-fixture\\Docs");
assert.equal(back, "C:\\Temp\\afm-fixture");
assert.equal(canGoForward.value, true);

const fwd = popForward("C:\\Temp\\afm-fixture");
assert.equal(fwd, "C:\\Temp\\afm-fixture\\Docs");
assert.equal(canGoBack.value, true);

beginHistoryNavigation();
recordNavHistory("C:\\Temp\\afm-fixture\\Docs", "C:\\Temp");
endHistoryNavigation();
// History jump must not push.
assert.equal(canGoForward.value, false);

const crumbs = buildBreadcrumbs("C:\\Temp\\afm-fixture\\Docs\\a");
assert.deepEqual(
  crumbs.map((c) => c.label),
  ["C:", "Temp", "afm-fixture", "Docs", "a"],
);
assert.equal(crumbs[0].path, "C:\\");
assert.equal(crumbs[3].path, "C:\\Temp\\afm-fixture\\Docs");

const unc = buildBreadcrumbs("\\\\server\\share\\folder");
assert.equal(unc[0].label, "\\\\server");
assert.equal(unc[1].label, "share");
assert.equal(unc[2].path, "\\\\server\\share\\folder");

assert.deepEqual(buildBreadcrumbs("afm://guide"), []);

const withRoot = buildNavigatorBreadcrumbs("C:\\Temp\\afm-fixture", "This PC");
assert.equal(withRoot[0].kind, "root");
assert.equal(withRoot[0].path, GUIDE_PATH);
assert.equal(withRoot[0].label, "This PC");
assert.equal(withRoot[1].label, "C:");
assert.equal(withRoot[2].path, "C:\\Temp");
assert.equal(withRoot[3].path, "C:\\Temp\\afm-fixture");

const guideOnly = buildNavigatorBreadcrumbs(GUIDE_PATH, "This PC");
assert.equal(guideOnly.length, 1);
assert.equal(guideOnly[0].kind, "root");

// AIA-149: Up from drive / share root lands on the same guide as new-tab.
assert.equal(navigableParentPath("D:\\"), GUIDE_PATH);
assert.equal(navigableParentPath("D:/"), GUIDE_PATH);
assert.equal(navigableParentPath("D:"), GUIDE_PATH);
assert.equal(navigableParentPath("C:\\Temp\\afm-fixture"), "C:\\Temp");
assert.equal(navigableParentPath("\\\\server\\share"), GUIDE_PATH);
assert.equal(navigableParentPath("\\\\server\\share\\docs"), "\\\\server\\share");
assert.equal(navigableParentPath(GUIDE_PATH), null);
assert.equal(
  buildNavigatorBreadcrumbs("D:\\", "This PC")[0].path,
  navigableParentPath("D:\\"),
);

resetNavHistory();
console.log("Nav history / breadcrumb checks OK (AIA-63).");
