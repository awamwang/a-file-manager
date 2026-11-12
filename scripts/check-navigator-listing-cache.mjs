// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-58: large-dir listing LRU + fresh short-circuit for tab switch. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const cache = readFileSync(join(root, "src/state/navigatorListingCache.ts"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const showHidden = readFileSync(join(root, "src/state/showHidden/index.ts"), "utf8");

assert.ok(cache.includes("LISTING_CACHE_MAX_ENTRIES"));
assert.ok(cache.includes("LISTING_CACHE_FRESH_MS"));
assert.ok(cache.includes("putCachedListing"));
assert.ok(cache.includes("isListingCacheFresh"));

assert.ok(browse.includes("navigatorListingCache"));
assert.ok(browse.includes("bypassListingCache"));
assert.ok(browse.includes("putCachedListing"));
assert.ok(
  browse.includes("isListingCacheFresh"),
  "fresh cache must short-circuit navigator_list on tab revisit",
);
assert.ok(
  /refreshActiveListing[\s\S]*bypassListingCache:\s*true/.test(browse),
  "F5 refresh must bypass listing cache",
);

assert.ok(
  view.includes("gridVirtualPaddingStyle"),
  "icon/tiles window uses padding offsets",
);
assert.ok(
  !view.includes("isGridIconOrTilesMode"),
  "icon/tiles virtualization re-enabled (FIL-58)",
);
assert.ok(
  view.includes("content-visibility: auto") || view.includes("content-visibility:auto"),
  "icon cells hint browser to skip offscreen work",
);

assert.ok(
  showHidden.includes("clearListingCache"),
  "showHidden toggle must drop listing cache",
);

const registerUrl = pathToFileURL(join(root, "scripts/register-ts-resolve.mjs")).href;
const unit = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/navigator-listing-cache-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "navigator-listing-cache-checks must pass");

console.log("navigator listing cache checks OK (FIL-58)");
