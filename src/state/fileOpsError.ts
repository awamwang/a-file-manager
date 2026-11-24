// SPDX-License-Identifier: GPL-3.0-or-later
/** Unified localized-error rendering for file operations.
 *
 *  The backend (`file_ops/mod.rs::classify_error` / `err_code`) attaches a
 *  machine-readable `errorCode` + `errorParams` to a `FileOpsResult` at the
 *  high-frequency conflict / validation / path sites, while keeping the English
 *  `error` string for logs and for any code the locale does not know. This
 *  module turns that contract back into a localized line, with a non-breaking
 *  fallback: when there is no known code (or the locale has no matching key)
 *  the raw English message wins, exactly as before. */

import { i18n } from "../i18n/index.ts";

export type FileOpsErrorShape = {
  errorCode?: string | null;
  errorParams?: Record<string, string> | null;
  error?: string | null;
};

/** True when the active locale actually defines `fileOps.error.<code>`. Uses the
 *  global composer's `te` when available; otherwise it cannot confirm a key and
 *  the caller falls through to the unresolved-translation probe. */
function hasErrorCodeKey(key: string): boolean {
  const te = (i18n.global as unknown as { te?: (k: string) => boolean }).te;
  if (typeof te === "function") {
    try {
      return te.call(i18n.global, key);
    } catch {
      return false;
    }
  }
  return false;
}

/** Localize a file-operation failure.
 *
 *  Resolution order:
 *  1. `errorCode` set AND the locale defines `fileOps.error.<code>` → localized
 *     line with `errorParams` interpolated (e.g. `「{name}」已存在。`).
 *  2. `errorCode` set but the key is missing → best-effort `t()`; if it returns
 *     the raw key (unresolved) treat as missing and fall through.
 *  3. no usable code → the English `error` if present, else `t(fallbackKey)`. */
export function localizeFileOpError(
  res: FileOpsErrorShape,
  t: (key: string, named?: Record<string, unknown>) => string,
  fallbackKey = "fileOps.error.io_failed",
): string {
  if (res.errorCode) {
    const key = `fileOps.error.${res.errorCode}`;
    const named = (res.errorParams ?? {}) as Record<string, unknown>;
    if (hasErrorCodeKey(key)) {
      return t(key, named);
    }
    // `te` unavailable (or unhelpful): try `t` and detect an unresolved key.
    const translated = t(key, named);
    if (translated && translated !== key) {
      return translated;
    }
  }
  return res.error || t(fallbackKey);
}
