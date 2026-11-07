// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-238: open-tab refresh after external folder changes (F5 / toolbar / focus). */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const browse = read("src/state/navigatorBrowse.ts");
assert.ok(
  browse.includes("export function refreshActiveListing"),
  "navigatorBrowse must export refreshActiveListing",
);
assert.ok(
  /recordVisit\s*=\s*options\?\.recordVisit\s*!==\s*false/.test(browse) ||
    browse.includes("recordVisit?: boolean"),
  "navigateTo must accept recordVisit option",
);
{
  const start = browse.indexOf("export function refreshActiveListing");
  assert.ok(start >= 0);
  const next = browse.indexOf("\nexport ", start + 1);
  const body = browse.slice(start, next > start ? next : browse.length);
  assert.ok(
    /void\s+navigateTo\(/.test(body),
    "refreshActiveListing must not await navigateTo (non-blocking)",
  );
  assert.ok(
    !/await\s+navigateTo\(/.test(body),
    "refreshActiveListing must not block on listing IPC",
  );
  assert.ok(
    body.includes("recordVisit: false"),
    "refresh must not inflate visit stats",
  );
  assert.ok(
    body.includes("recordHistory: false"),
    "refresh must not pollute back stack",
  );
  assert.ok(
    body.includes("syncOpenState: false"),
    "refresh must not rewrite OpenState tab path",
  );
}

const nav = read("src/views/NavigatorView.vue");
assert.ok(nav.includes("refreshActiveListing"), "NavigatorView must use refreshActiveListing");
assert.ok(
  nav.includes('registerShortcutHandler("navigator.refresh"'),
  "F5 shortcut must stay wired",
);
assert.ok(nav.includes("onFocusChanged"), "Tauri window focus must re-list");
assert.ok(nav.includes("refreshListingOnFocus"), "focus refresh helper required");
assert.ok(
  /navigator\.refresh[\s\S]*?refreshActiveListing\(/.test(nav),
  "F5 handler must call refreshActiveListing without awaiting list",
);

const shell = read("src/components/shell/AppShell.vue");
assert.ok(shell.includes("onRefreshListing"), "toolbar Refresh handler missing");
assert.ok(shell.includes('name="refresh"'), "toolbar Refresh icon missing");
assert.ok(
  shell.includes('t("chrome.refresh")') || shell.includes("t('chrome.refresh')"),
  "toolbar Refresh aria-label missing",
);
assert.ok(
  shell.includes("refreshActiveListing"),
  "toolbar must call shared refreshActiveListing",
);

const glyphs = read("src/components/icons/glyphs.ts");
assert.ok(glyphs.includes('"refresh"') || glyphs.includes("| \"refresh\""), "refresh glyph id");
assert.ok(glyphs.includes("refresh:"), "refresh glyph entry");
assert.ok(glyphs.includes("\\uE72C") || glyphs.includes("\uE72C"), "Segoe Refresh codepoint");

const en = read("src/i18n/locales/en.json");
assert.ok(en.includes('"refreshTitle"'), "en chrome.refreshTitle missing");

const shortcuts = read("src/shortcuts/registry.ts");
assert.ok(shortcuts.includes("navigator.refresh"));
assert.ok(/chord:\s*\{\s*key:\s*"f5"\s*\}/.test(shortcuts), "F5 chord required");

console.log("open-tab refresh checks OK (AIA-238).");
