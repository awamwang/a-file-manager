// SPDX-License-Identifier: GPL-3.0-or-later

import { createI18n } from "vue-i18n";
import en from "./locales/en.json" with { type: "json" };
import zhCN from "./locales/zh-CN.json" with { type: "json" };
import { resolveAppLocale, type AppLocale, SUPPORTED_LOCALES, isAppLocale } from "./locale";

export { SUPPORTED_LOCALES, isAppLocale, resolveAppLocale, type AppLocale };

export const i18n = createI18n({
  legacy: false,
  locale: "en",
  fallbackLocale: "en",
  messages: {
    en,
    "zh-CN": zhCN,
  },
});

export function applyAppLocale(locale: AppLocale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale === "zh-CN" ? "zh-CN" : "en";
}

export function applyAppLocaleFromValues(values: Record<string, unknown>): void {
  applyAppLocale(resolveAppLocale(values["core.language"]));
}
