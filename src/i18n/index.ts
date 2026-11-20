// SPDX-License-Identifier: GPL-3.0-or-later

import { createI18n } from "vue-i18n";
import en from "./locales/en.json" with { type: "json" };
import zhCN from "./locales/zh-CN.json" with { type: "json" };
import zhTW from "./locales/zh-TW.json" with { type: "json" };
import ja from "./locales/ja.json" with { type: "json" };
import de from "./locales/de.json" with { type: "json" };
import fr from "./locales/fr.json" with { type: "json" };
import {
  resolveAppLocale,
  type AppLocale,
  SUPPORTED_LOCALES,
  isAppLocale,
  htmlLangForLocale,
  SYSTEM_LOCALE_OPTION,
  detectSystemLocale,
  matchSystemLocale,
} from "./locale.ts";
import { localizeCatalog, type MessageCatalog } from "./platformTerms.ts";
import { isMacPlatform } from "../config/platform.ts";

export {
  SUPPORTED_LOCALES,
  isAppLocale,
  resolveAppLocale,
  htmlLangForLocale,
  SYSTEM_LOCALE_OPTION,
  detectSystemLocale,
  matchSystemLocale,
  type AppLocale,
};

/**
 * Keep each catalog's inferred shape: vue-i18n derives `global.locale` and the
 * message-key types from `messages`, so widening it to `Record<string, unknown>`
 * breaks `vue-tsc`. The runtime value is a plain localized catalog.
 */
function localized<T>(catalog: T, mac: boolean): T {
  return localizeCatalog(catalog as unknown as MessageCatalog, mac) as unknown as T;
}

// `isMacPlatform()` answers synchronously via the UA fallback until the
// capability IPC lands; `applyPlatformTerms()` re-merges once it does.
const initialMac = isMacPlatform();

export const i18n = createI18n({
  legacy: false,
  // Placeholder initial value; bootstrap() calls applyAppLocaleFromValues()
  // before mount so the first paint already shows the resolved locale
  // (including the `system` → navigator.languages detection path).
  locale: "en",
  fallbackLocale: "en",
  messages: {
    en: localized(en, initialMac),
    "zh-CN": localized(zhCN, initialMac),
    "zh-TW": localized(zhTW, initialMac),
    ja: localized(ja, initialMac),
    de: localized(de, initialMac),
    fr: localized(fr, initialMac),
  },
});

/**
 * Re-merge copy after `hydratePlatform()` resolves the authoritative host.
 * `setLocaleMessage` keeps the active locale and is reactive, so already
 * rendered labels update without a reload.
 */
export function applyPlatformTerms(): void {
  const mac = isMacPlatform();
  i18n.global.setLocaleMessage("en", localized(en, mac));
  i18n.global.setLocaleMessage("zh-CN", localized(zhCN, mac));
  i18n.global.setLocaleMessage("zh-TW", localized(zhTW, mac));
  i18n.global.setLocaleMessage("ja", localized(ja, mac));
  i18n.global.setLocaleMessage("de", localized(de, mac));
  i18n.global.setLocaleMessage("fr", localized(fr, mac));
}

export function applyAppLocale(locale: AppLocale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = htmlLangForLocale(locale);
}

export function applyAppLocaleFromValues(values: Record<string, unknown>): void {
  applyAppLocale(resolveAppLocale(values["core.language"]));
}
