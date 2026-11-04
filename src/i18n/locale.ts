// SPDX-License-Identifier: GPL-3.0-or-later

export const SUPPORTED_LOCALES = ["en", "zh-CN", "ja", "de", "fr"] as const;
export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

export function isAppLocale(value: unknown): value is AppLocale {
  return (
    typeof value === "string" &&
    (SUPPORTED_LOCALES as readonly string[]).includes(value)
  );
}

export function resolveAppLocale(value: unknown): AppLocale {
  return isAppLocale(value) ? value : "en";
}

/** BCP 47 tag for `document.documentElement.lang`. */
export function htmlLangForLocale(locale: AppLocale): string {
  return locale;
}
