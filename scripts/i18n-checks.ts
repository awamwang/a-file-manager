// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  isAppLocale,
  resolveAppLocale,
  SUPPORTED_LOCALES,
} from "../src/i18n/locale.ts";
import {
  SCHEMA_ENUM_OPTIONS,
  SCHEMA_SETTING_KEYS,
  settingEnumOptionPath,
  settingFieldMessagePath,
} from "../src/i18n/settingsFields.ts";

assert.deepEqual([...SUPPORTED_LOCALES], ["en", "zh-CN"]);
assert.equal(isAppLocale("zh-CN"), true);
assert.equal(isAppLocale("fr"), false);
assert.equal(resolveAppLocale("zh-CN"), "zh-CN");
assert.equal(resolveAppLocale("nope"), "en");
assert.equal(resolveAppLocale(undefined), "en");

console.log("i18n locale helper checks OK.");

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

function lookup(messages: Record<string, unknown>, path: string): unknown {
  return path.split(".").reduce<unknown>((node, part) => {
    if (node && typeof node === "object" && part in (node as object)) {
      return (node as Record<string, unknown>)[part];
    }
    return undefined;
  }, messages);
}

for (const localeName of ["en", "zh-CN"] as const) {
  const messages = localeName === "en" ? en : zh;
  for (const key of SCHEMA_SETTING_KEYS) {
    for (const part of ["title", "description"] as const) {
      const path = settingFieldMessagePath(key, part);
      const value = lookup(messages, path);
      assert.equal(
        typeof value,
        "string",
        `${localeName} missing non-empty ${path}`,
      );
      assert.ok(String(value).trim().length > 0, `${localeName} empty ${path}`);
    }
    const options = SCHEMA_ENUM_OPTIONS[key];
    if (!options) continue;
    for (const option of options) {
      const path = settingEnumOptionPath(key, option);
      const value = lookup(messages, path);
      assert.equal(
        typeof value,
        "string",
        `${localeName} missing enum label ${path}`,
      );
      assert.ok(String(value).trim().length > 0, `${localeName} empty ${path}`);
    }
  }
}

// zh-CN field titles must not fall back to the English schema copy for Core language.
assert.notEqual(
  lookup(zh, "settings.fields.core.language.title"),
  "Language",
  "zh-CN core.language.title must be localized",
);
assert.notEqual(
  lookup(zh, "settings.fields.core.showHidden.title"),
  "Show hidden items",
  "zh-CN core.showHidden.title must be localized",
);

console.log(
  `i18n settings field overlay OK (${SCHEMA_SETTING_KEYS.length} schema keys × title/description).`,
);
