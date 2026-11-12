// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const guide = readFileSync(
  join(root, "src/components/navigator/NavigatorGuide.vue"),
  "utf8",
);
const providers = readFileSync(join(root, "src/views/ProvidersView.vue"), "utf8");
const history = readFileSync(join(root, "src/state/navHistory.ts"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const navigatorView = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const registerUrl = pathToFileURL(join(root, "scripts/register-ts-resolve.mjs")).href;

assert.ok(browse.includes("isProviderPath"));
assert.ok(browse.includes("loadProviderListing"));
assert.ok(browse.includes("openProviderEntry"));
assert.ok(browse.includes("isBlockedSearchProvider"));
assert.ok(browse.includes("PROVIDER_SEARCH_BLOCKED"));
const navigateToSrc = browse.slice(
  browse.indexOf("export async function navigateTo"),
);
const epochBump = navigateToSrc.indexOf("++browseEpoch");
assert.ok(epochBump > 0);
const navigateToBeforeEpoch = navigateToSrc.slice(0, epochBump);
assert.ok(navigateToBeforeEpoch.includes("isBlockedSearchProvider"));
assert.ok(navigateToBeforeEpoch.includes("PROVIDER_SEARCH_BLOCKED"));
assert.ok(!navigateToBeforeEpoch.includes("listing.value = null"));
const navigateSecondarySrc = browse.slice(
  browse.indexOf("export async function navigateSecondary"),
  browse.indexOf("export async function navigateTo"),
);
assert.ok(navigateSecondarySrc.includes("isBlockedSearchProvider"));
assert.ok(navigateSecondarySrc.includes("PROVIDER_SEARCH_BLOCKED"));
assert.ok(
  navigateSecondarySrc.indexOf("isBlockedSearchProvider") <
    navigateSecondarySrc.indexOf("secondaryListing.value = null"),
);
assert.ok(guide.includes("PROVIDER_ROOT"));
assert.ok(guide.includes("listProviderBackends"));
assert.ok(guide.includes("guide.sections.providers"));
assert.ok(providers.includes("openInExplorer"));
assert.ok(history.includes("buildProviderBreadcrumbs"));
assert.equal(en.guide.sections.providers, "Providers");
assert.equal(en.providers.openInExplorer, "Open in Explorer");
assert.equal(
  en.providers.searchBlocked,
  "Everything is available via Search. The search provider backend is disabled here.",
);
assert.equal(en.chrome.providerQueryApply, "Apply");
assert.equal(zh.chrome.providerQueryApply, "查询");
assert.ok(shell.includes("provider-query-bar"));
assert.ok(shell.includes("submitProviderQuery"));
assert.ok(shell.includes("withProviderQuery"));
assert.ok(shell.includes("getProviderQuery"));
assert.ok(navigatorView.includes("providers.searchBlocked"));
assert.ok(navigatorView.includes("PROVIDER_SEARCH_BLOCKED"));

const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/provider-path-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("Provider path shell checks OK");
