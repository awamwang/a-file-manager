// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  isAppLocale,
  resolveAppLocale,
  SUPPORTED_LOCALES,
} from "../src/i18n/locale.ts";

assert.deepEqual([...SUPPORTED_LOCALES], ["en", "zh-CN"]);
assert.equal(isAppLocale("zh-CN"), true);
assert.equal(isAppLocale("fr"), false);
assert.equal(resolveAppLocale("zh-CN"), "zh-CN");
assert.equal(resolveAppLocale("nope"), "en");
assert.equal(resolveAppLocale(undefined), "en");

console.log("i18n locale helper checks OK.");
