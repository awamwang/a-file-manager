// SPDX-License-Identifier: GPL-3.0-or-later

import { createI18n } from "vue-i18n";
import en from "./locales/en.json" with { type: "json" };
import zhCN from "./locales/zh-CN.json" with { type: "json" };
import ja from "./locales/ja.json" with { type: "json" };
import de from "./locales/de.json" with { type: "json" };
import fr from "./locales/fr.json" with { type: "json" };
import {
  resolveAppLocale,
  type AppLocale,
  SUPPORTED_LOCALES,
  isAppLocale,
  htmlLangForLocale,
} from "./locale";

export {
  SUPPORTED_LOCALES,
  isAppLocale,
  resolveAppLocale,
  htmlLangForLocale,
  type AppLocale,
};

export const i18n = createI18n({
  legacy: false,
  locale: "en",
  fallbackLocale: "en",
  messages: {
    en,
    "zh-CN": zhCN,
    ja,
    de,
    fr,
  },
});

export function applyAppLocale(locale: AppLocale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = htmlLangForLocale(locale);
}

export function applyAppLocaleFromValues(values: Record<string, unknown>): void {
  applyAppLocale(resolveAppLocale(values["core.language"]));
}
