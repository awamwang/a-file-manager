// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-70: dark theme content-page walkthrough gates. */
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const tokens = readFileSync(join(root, "src/styles/fluent-tokens.css"), "utf8");
const chrome = readFileSync(join(root, "src/state/uiChrome.ts"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");

assert.ok(tokens.includes('html[data-theme="dark"]'));
assert.ok(tokens.includes("color-scheme: dark"));
assert.ok(tokens.includes("--fluent-color-fill-subtle: #2c2c2c"));
assert.ok(tokens.includes(".fluent-page"));
assert.ok(chrome.includes("colorScheme"));
assert.ok(shell.includes("fluent-color-fill-subtle"));

const viewsDir = join(root, "src/views");
const views = readdirSync(viewsDir).filter((f) => f.endsWith(".vue"));
assert.ok(views.length >= 8, "expected content views");

const hardcoded = /(?:background|color|border(?:-color)?)\s*:\s*#(?:fff|ffffff|000|000000)\b/i;
for (const file of views) {
  const text = readFileSync(join(viewsDir, file), "utf8");
  assert.ok(
    text.includes("fluent-page") || text.includes("fluent-"),
    `${file} should use Fluent primitives`,
  );
  assert.ok(!hardcoded.test(text), `${file} has hardcoded black/white paint`);
}

console.log(
  `Dark theme walkthrough OK (AIA-70): ${views.length} views, tokens+color-scheme+content pane).`,
);
