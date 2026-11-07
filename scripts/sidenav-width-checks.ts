// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  DEFAULT_SIDENAV_WIDTH,
  MAX_SIDENAV_WIDTH,
  MIN_SIDENAV_WIDTH,
  MIN_WORKSPACE_WIDTH,
  clampSidenavWidth,
} from "../src/state/sidenavWidth.ts";

assert.equal(DEFAULT_SIDENAV_WIDTH, 200);
assert.equal(MIN_SIDENAV_WIDTH, 160);
assert.equal(MAX_SIDENAV_WIDTH, 480);

assert.equal(clampSidenavWidth(DEFAULT_SIDENAV_WIDTH), DEFAULT_SIDENAV_WIDTH);
assert.equal(clampSidenavWidth(120), MIN_SIDENAV_WIDTH);
assert.equal(clampSidenavWidth(900), MAX_SIDENAV_WIDTH);
assert.equal(clampSidenavWidth(Number.NaN), DEFAULT_SIDENAV_WIDTH);
assert.equal(clampSidenavWidth(201.6), 202);

// Narrow viewport keeps workspace usable.
assert.equal(clampSidenavWidth(480, 500), 500 - MIN_WORKSPACE_WIDTH);
assert.equal(clampSidenavWidth(100, 500), MIN_SIDENAV_WIDTH);
assert.ok(clampSidenavWidth(480, 500) >= MIN_SIDENAV_WIDTH);

console.log("sidenav-width checks OK.");
