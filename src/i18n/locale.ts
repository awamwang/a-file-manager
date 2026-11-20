// SPDX-License-Identifier: GPL-3.0-or-later

export const SUPPORTED_LOCALES = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"] as const;
export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

/** Stored `core.language` sentinel: resolve to the host's UI language at startup. */
export const SYSTEM_LOCALE_OPTION = "system";

export function isAppLocale(value: unknown): value is AppLocale {
  return (
    typeof value === "string" &&
    (SUPPORTED_LOCALES as readonly string[]).includes(value)
  );
}

/**
 * Map ordered BCP 47 candidates (most-preferred first) to a supported catalog.
 * Pure and synchronous so it is unit-testable without a live `navigator`.
 * All Han variants collapse to a single catalog: `zh-Hant`/`zh-TW`/`zh-HK`
 * resolve to `zh-TW`, other `zh*` to `zh-CN`.
 */
export function matchSystemLocale(candidates: readonly string[]): AppLocale {
  for (const raw of candidates) {
    const tag = raw.toLowerCase();
    if (tag.startsWith("zh")) {
      return /(tw|hk|mo|hant)/.test(tag) ? "zh-TW" : "zh-CN";
    }
    if (tag.startsWith("ja")) return "ja";
    if (tag.startsWith("de")) return "de";
    if (tag.startsWith("fr")) return "fr";
    if (tag.startsWith("en")) return "en";
  }
  return "en";
}

/**
 * Best-effort host UI language. WebView2 mirrors the OS display language via
 * `navigator.languages`; missing/foreign locales fall back to `en`.
 */
export function detectSystemLocale(): AppLocale {
  const nav = globalThis.navigator as
    | { languages?: readonly string[]; language?: string }
    | undefined;
  const list = nav?.languages?.length
    ? nav.languages
    : nav?.language
      ? [nav.language]
      : [];
  return matchSystemLocale(list);
}

export function resolveAppLocale(
  value: unknown,
  systemLocale: AppLocale = detectSystemLocale(),
): AppLocale {
  if (value === SYSTEM_LOCALE_OPTION) return systemLocale;
  return isAppLocale(value) ? value : "en";
}

/** BCP 47 tag for `document.documentElement.lang`. */
export function htmlLangForLocale(locale: AppLocale): string {
  return locale;
}
