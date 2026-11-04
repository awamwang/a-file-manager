// SPDX-License-Identifier: GPL-3.0-or-later

export const SUPPORTED_LOCALES = ["en", "zh-CN"] as const;
export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

export function isAppLocale(value: unknown): value is AppLocale {
  return value === "en" || value === "zh-CN";
}

export function resolveAppLocale(value: unknown): AppLocale {
  return isAppLocale(value) ? value : "en";
}
