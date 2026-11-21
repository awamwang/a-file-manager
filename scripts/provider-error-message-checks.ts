// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the provider error classifier. Pinned by scripts/check-provider-error-hint.mjs.
// errorMessage.ts has zero runtime imports, so this exercises the REAL classification —
// the order-sensitive rule table is exactly what a careless refactor (reorder/regex edit) breaks.
import assert from "node:assert/strict";
import {
  classifyProviderError,
  providerErrorI18nKey,
  shouldShowProviderErrorRaw,
} from "../src/state/provider/errorMessage.ts";

// Empty / whitespace / nullish collapse to generic (never throws on missing stderr).
for (const blank of ["", "   ", "\n", null, undefined]) {
  assert.equal(classifyProviderError(blank as string | null | undefined), "generic");
}

// Real-world stderr fragments map to the intended semantic kind (case-insensitive substrings).
assert.equal(
  classifyProviderError(
    "Propfind: connectex: No connection could be made because the target machine actively refused it.",
  ),
  "connectionRefused",
);
assert.equal(classifyProviderError("dial tcp 127.0.0.1:5244: connect: refused"), "connectionRefused");
assert.equal(classifyProviderError("rclone not found in PATH"), "rcloneMissing");
assert.equal(classifyProviderError("Failed to init: no rclone remotes configured"), "notConfigured");
assert.equal(classifyProviderError("request timed out after 30s"), "timeout");
assert.equal(classifyProviderError("context deadline exceeded"), "timeout");
assert.equal(classifyProviderError("HTTP 403 Forbidden"), "auth");
assert.equal(classifyProviderError("signature does not match"), "auth");
assert.equal(classifyProviderError("directory not found"), "notFound");
assert.equal(classifyProviderError("HTTP 404"), "notFound");
assert.equal(classifyProviderError("PROVIDER_SEARCH_BLOCKED"), "searchBlocked");
assert.equal(classifyProviderError("something entirely unrecognised"), "generic");

// Order sensitivity: a connection message that ALSO contains "not found" must stay connectionRefused.
assert.equal(
  classifyProviderError("connectex: target machine refused, host not found"),
  "connectionRefused",
);

// i18n keys: searchBlocked reuses the existing string; everything else is namespaced.
assert.equal(providerErrorI18nKey("searchBlocked"), "providers.searchBlocked");
assert.equal(providerErrorI18nKey("timeout"), "providers.error.timeout");
assert.equal(providerErrorI18nKey("generic"), "providers.error.generic");

// Raw stderr stays visible for every real error, hidden only for the synthetic internal code.
assert.equal(shouldShowProviderErrorRaw("searchBlocked"), false);
assert.equal(shouldShowProviderErrorRaw("connectionRefused"), true);
assert.equal(shouldShowProviderErrorRaw("generic"), true);

console.log(
  "provider-error-message-checks OK (order-sensitive classification, nullish->generic, raw stays visible).",
);
