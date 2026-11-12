// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-58 unit checks for navigator listing LRU. */
import assert from "node:assert/strict";
import {
  clearListingCache,
  getCachedListing,
  isListingCacheFresh,
  LISTING_CACHE_MAX_ENTRIES,
  listingCacheKey,
  listingCacheSize,
  putCachedListing,
} from "../src/state/navigatorListingCache.ts";

type SampleListing = {
  path: string;
  dirCount: number;
  fileCount: number;
  entries: Array<{ name: string; kind: string; size: number; path: string }>;
};

function sample(path: string, n: number): SampleListing {
  return {
    path,
    dirCount: 0,
    fileCount: n,
    entries: Array.from({ length: n }, (_, i) => ({
      name: `f${i}.txt`,
      kind: "file",
      size: i,
      path: `${path}\\f${i}.txt`,
    })),
  };
}

clearListingCache();
assert.equal(listingCacheSize(), 0);

const keyA = listingCacheKey("G:\\Projects\\Tools\\a-file-manager\\.perf-fixtures\\large", false);
const keyB = listingCacheKey("G:\\Projects\\Tools\\a-file-manager\\.perf-fixtures\\med", false);
putCachedListing(keyA, sample("G:\\large", 5000));
assert.ok(getCachedListing<SampleListing>(keyA));
assert.ok(isListingCacheFresh(keyA));
assert.equal(getCachedListing(keyB), null);

for (let i = 0; i < LISTING_CACHE_MAX_ENTRIES + 3; i += 1) {
  putCachedListing(listingCacheKey(`G:\\tmp\\dir${i}`, false), sample(`G:\\tmp\\dir${i}`, 10));
}
assert.ok(listingCacheSize() <= LISTING_CACHE_MAX_ENTRIES);
assert.equal(getCachedListing(keyA), null, "oldest entries must evict under max");

const staleKey = listingCacheKey("G:\\stale", true);
putCachedListing(staleKey, sample("G:\\stale", 3), Date.now() - 120_000);
assert.ok(getCachedListing(staleKey));
assert.equal(isListingCacheFresh(staleKey), false);

clearListingCache();
console.log("navigator-listing-cache-checks OK");
