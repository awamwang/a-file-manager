// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  DEFAULT_DETAILS_PANE_WIDTH,
  DEFAULT_PREVIEW_PANE_WIDTH,
  SIDE_PANE_MAX_WIDTH,
  SIDE_PANE_MIN_WIDTH,
  SIDE_PANE_MIN_WORKSPACE,
  clampSidePaneWidth,
} from "../src/state/sidePaneWidth.ts";
import { DUAL_PANE_MIN_PANE_PX } from "../src/state/dualPaneLayout.ts";

assert.equal(SIDE_PANE_MIN_WIDTH, 240);
assert.equal(SIDE_PANE_MIN_WIDTH, DUAL_PANE_MIN_PANE_PX, "align with AIA-252 dual-pane floor");
assert.equal(SIDE_PANE_MAX_WIDTH, 560);
assert.equal(DEFAULT_PREVIEW_PANE_WIDTH, 352);
assert.equal(DEFAULT_DETAILS_PANE_WIDTH, 320);

assert.equal(
  clampSidePaneWidth(DEFAULT_PREVIEW_PANE_WIDTH, DEFAULT_PREVIEW_PANE_WIDTH),
  DEFAULT_PREVIEW_PANE_WIDTH,
);
assert.equal(clampSidePaneWidth(100, DEFAULT_PREVIEW_PANE_WIDTH), SIDE_PANE_MIN_WIDTH);
assert.equal(clampSidePaneWidth(900, DEFAULT_DETAILS_PANE_WIDTH), SIDE_PANE_MAX_WIDTH);
assert.equal(clampSidePaneWidth(Number.NaN, DEFAULT_PREVIEW_PANE_WIDTH), DEFAULT_PREVIEW_PANE_WIDTH);
assert.equal(clampSidePaneWidth(321.6, DEFAULT_DETAILS_PANE_WIDTH), 322);

// Narrow viewport: workspace reserve can sit below the pane floor → floor wins.
assert.equal(
  clampSidePaneWidth(560, DEFAULT_PREVIEW_PANE_WIDTH, 500),
  SIDE_PANE_MIN_WIDTH,
);
assert.ok(500 - SIDE_PANE_MIN_WORKSPACE < SIDE_PANE_MIN_WIDTH);
// Wider viewport still honors workspace reserve.
assert.equal(
  clampSidePaneWidth(560, DEFAULT_PREVIEW_PANE_WIDTH, 900),
  SIDE_PANE_MAX_WIDTH,
);
assert.equal(
  clampSidePaneWidth(560, DEFAULT_PREVIEW_PANE_WIDTH, 700),
  700 - SIDE_PANE_MIN_WORKSPACE,
);
assert.equal(clampSidePaneWidth(100, DEFAULT_PREVIEW_PANE_WIDTH, 500), SIDE_PANE_MIN_WIDTH);

console.log("side-pane-width checks OK.");
