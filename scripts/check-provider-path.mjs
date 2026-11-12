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
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const registerUrl = pathToFileURL(join(root, "scripts/register-ts-resolve.mjs")).href;

assert.ok(browse.includes("isProviderPath"));
assert.ok(browse.includes("loadProviderListing"));
assert.ok(browse.includes("openProviderEntry"));
assert.ok(browse.includes("isBlockedSearchProvider"));
assert.ok(browse.includes("PROVIDER_SEARCH_BLOCKED"));
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
