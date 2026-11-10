// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  DEFAULT_WINDOW_MIN_WIDTH_PX,
  DUAL_PANE_GAP_PX,
  DUAL_PANE_MIN_PANE_PX,
  DUAL_PANE_WINDOW_MIN_WIDTH_PX,
  dualPaneFitsInnerWidth,
  windowMinWidthForDualPane,
} from "../src/state/dualPane/layout.ts";

assert.equal(DEFAULT_WINDOW_MIN_WIDTH_PX, 330);
assert.equal(DUAL_PANE_MIN_PANE_PX, 240);
assert.equal(DUAL_PANE_GAP_PX, 8);
assert.equal(
  DUAL_PANE_WINDOW_MIN_WIDTH_PX,
  DUAL_PANE_MIN_PANE_PX * 2 + DUAL_PANE_GAP_PX + 232,
);
assert.equal(windowMinWidthForDualPane(false), DEFAULT_WINDOW_MIN_WIDTH_PX);
assert.equal(windowMinWidthForDualPane(true), DUAL_PANE_WINDOW_MIN_WIDTH_PX);
assert.equal(dualPaneFitsInnerWidth(DUAL_PANE_WINDOW_MIN_WIDTH_PX - 1), false);
assert.equal(dualPaneFitsInnerWidth(DUAL_PANE_WINDOW_MIN_WIDTH_PX), true);
assert.equal(dualPaneFitsInnerWidth(Number.NaN), false);

console.log("dual-pane runtime checks OK (AIA-252).");
