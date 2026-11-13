// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";

const dataset: Record<string, string | undefined> = {};
const style: { colorScheme?: string } = {};

(globalThis as {
  document: { documentElement: { dataset: typeof dataset; style: typeof style } };
}).document = {
  documentElement: {
    dataset,
    style,
  },
};

const { applyUiChrome, getUiTheme, getListDensity } = await import(
  "../src/state/uiChrome.ts"
);

applyUiChrome("light", "standard");
assert.equal(dataset.theme, "light");
assert.equal(style.colorScheme, "light");
assert.equal(dataset.listDensity, undefined);
assert.equal(getUiTheme(), "light");
assert.equal(getListDensity(), "standard");

applyUiChrome("dark", "compact");
assert.equal(dataset.theme, "dark");
assert.equal(style.colorScheme, "dark");
assert.equal(dataset.listDensity, "compact");
assert.equal(getListDensity(), "compact");

applyUiChrome("system", "standard");
assert.equal(dataset.theme, undefined);
assert.equal(style.colorScheme, "");
assert.equal(dataset.listDensity, undefined);
assert.equal(getUiTheme(), "system");

console.log("uiChrome apply checks OK (theme + listDensity → html dataset).");
