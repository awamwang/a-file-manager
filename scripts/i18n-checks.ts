// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  htmlLangForLocale,
  isAppLocale,
  resolveAppLocale,
  matchSystemLocale,
  SUPPORTED_LOCALES,
} from "../src/i18n/locale.ts";
import {
  SCHEMA_ENUM_OPTIONS,
  SCHEMA_SETTING_KEYS,
  settingEnumOptionPath,
  settingFieldMessagePath,
} from "../src/i18n/settingsFields.ts";

assert.deepEqual([...SUPPORTED_LOCALES], ["en", "zh-CN", "zh-TW", "ja", "de", "fr"]);
assert.equal(isAppLocale("zh-CN"), true);
assert.equal(isAppLocale("zh-TW"), true);
assert.equal(isAppLocale("ja"), true);
assert.equal(isAppLocale("de"), true);
assert.equal(isAppLocale("fr"), true);
assert.equal(isAppLocale("ru"), false);
// "system" is a stored sentinel, not a concrete catalog.
assert.equal(isAppLocale("system"), false);
assert.equal(resolveAppLocale("fr"), "fr");
assert.equal(resolveAppLocale("nope"), "en");
assert.equal(resolveAppLocale(undefined), "en");
assert.equal(resolveAppLocale("system", "de"), "de");
assert.equal(matchSystemLocale(["de-AT", "en-US"]), "de");
assert.equal(matchSystemLocale(["zh-TW", "zh-Hant"]), "zh-TW");
assert.equal(matchSystemLocale(["zh-HK"]), "zh-TW");
assert.equal(matchSystemLocale(["zh-CN", "zh"]), "zh-CN");
assert.equal(matchSystemLocale(["en-GB"]), "en");
assert.equal(matchSystemLocale(["ko-KR", "en"]), "en");
assert.equal(htmlLangForLocale("ja"), "ja");
assert.equal(htmlLangForLocale("zh-CN"), "zh-CN");
assert.equal(htmlLangForLocale("zh-TW"), "zh-TW");

console.log("i18n locale helper checks OK.");

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const localeIds = [...SUPPORTED_LOCALES];
const catalogs = Object.fromEntries(
  localeIds.map((id) => [
    id,
    JSON.parse(readFileSync(join(root, `src/i18n/locales/${id}.json`), "utf8")),
  ]),
);

function lookup(messages: Record<string, unknown>, path: string): unknown {
  return path.split(".").reduce<unknown>((node, part) => {
    if (node && typeof node === "object" && part in (node as object)) {
      return (node as Record<string, unknown>)[part];
    }
    return undefined;
  }, messages);
}

for (const localeName of localeIds) {
  const messages = catalogs[localeName] as Record<string, unknown>;
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

// Localized titles must not fall back to English schema copy.
assert.notEqual(
  lookup(catalogs["zh-CN"] as Record<string, unknown>, "settings.fields.core.language.title"),
  "Language",
  "zh-CN core.language.title must be localized",
);
assert.notEqual(
  lookup(catalogs["zh-TW"] as Record<string, unknown>, "settings.fields.core.language.title"),
  "Language",
  "zh-TW core.language.title must be localized",
);
assert.notEqual(
  lookup(catalogs.ja as Record<string, unknown>, "settings.fields.core.language.title"),
  "Language",
  "ja core.language.title must be localized",
);
assert.notEqual(
  lookup(catalogs.de as Record<string, unknown>, "settings.fields.core.showHidden.title"),
  "Show hidden items",
  "de core.showHidden.title must be localized",
);
assert.notEqual(
  lookup(catalogs.fr as Record<string, unknown>, "settings.fields.core.showHidden.title"),
  "Show hidden items",
  "fr core.showHidden.title must be localized",
);

// Project side panel: CONTEXT is the thing itself, not "notes / 备注 / メモ / Notizen".
const notesWording: Record<string, string> = {
  en: "notes",
  "zh-CN": "备注",
  "zh-TW": "備註",
  ja: "メモ",
  de: "notizen",
  fr: "notes",
};
for (const localeName of localeIds) {
  const messages = catalogs[localeName] as Record<string, unknown>;
  assert.equal(
    lookup(messages, "projects.context"),
    "CONTEXT",
    `${localeName} projects.context must be uppercase CONTEXT`,
  );
  for (const key of ["sidePanelTitle", "toggleSidePanel"] as const) {
    const value = String(lookup(messages, `projects.${key}`) ?? "");
    assert.ok(
      value.includes("CONTEXT"),
      `${localeName} projects.${key} must name CONTEXT explicitly`,
    );
    assert.ok(
      !value.toLowerCase().includes(notesWording[localeName].toLowerCase()),
      `${localeName} projects.${key} still uses notes wording: ${value}`,
    );
  }
}
console.log("i18n project CONTEXT wording OK (6 locales: section + side panel title).");

console.log(
  `i18n settings field overlay OK (${SCHEMA_SETTING_KEYS.length} schema keys × title/description × ${localeIds.length} locales).`,
);
