// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  DETAILS_HEADER_HEIGHT_PX,
  estimatePlaceholderCount,
  estimateRowHeightPx,
  iconSlotCssSize,
  LIST_ROW_HEIGHT_PX,
  PLACEHOLDER_COUNT_CAP,
  PLACEHOLDER_FALLBACK_COUNT,
  PLACEHOLDER_ICON_SIZE,
} from "../src/state/listPlaceholders.ts";

assert.equal(estimateRowHeightPx("details"), LIST_ROW_HEIGHT_PX);
assert.equal(estimateRowHeightPx("list"), LIST_ROW_HEIGHT_PX);
assert.equal(
  estimateRowHeightPx("iconsMedium"),
  PLACEHOLDER_ICON_SIZE.iconsMedium + 64,
);
assert.equal(iconSlotCssSize("details"), "16px");
assert.equal(iconSlotCssSize("iconsLarge"), "96px");
assert.equal(iconSlotCssSize("iconsExtraLarge"), "256px");

assert.equal(estimatePlaceholderCount("details", 0, 0), PLACEHOLDER_FALLBACK_COUNT);
assert.equal(estimatePlaceholderCount("iconsMedium", -1, 400), PLACEHOLDER_FALLBACK_COUNT);

const detailsCount = estimatePlaceholderCount("details", 800, 320);
assert.equal(
  detailsCount,
  Math.ceil((320 - DETAILS_HEADER_HEIGHT_PX) / LIST_ROW_HEIGHT_PX),
);

const iconCount = estimatePlaceholderCount("iconsMedium", 480, 400);
// 480 / 96 = 5 cols; row ≈ 64+64=128 → 4 rows → 20 cells.
assert.equal(iconCount, 20);

const capped = estimatePlaceholderCount("list", 800, 50_000);
assert.equal(capped, PLACEHOLDER_COUNT_CAP);

console.log("list-placeholders checks OK (AIA-267).");
