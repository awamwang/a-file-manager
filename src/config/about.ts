// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Static metadata for the About / 关于 settings page.
 *
 * Keep display text localizable via `about.*` i18n keys; this module only owns
 * facts that are not translated (URLs, the author handle, license id, stack).
 * The runtime version comes from `health_get`, never hardcoded here.
 */

export const ABOUT_AUTHOR = {
  /** Display name shown on the About page (also the clickable label). */
  name: "Awam M Wang",
  /** Target opened when the author name is clicked. */
  homepage: "https://github.com/awamwang",
} as const;

export const ABOUT_REPO_URL = "https://github.com/awamwang/a-file-manager";

export const ABOUT_LINKS = {
  /** Project repository / homepage. */
  repository: ABOUT_REPO_URL,
  /** Latest installer download. */
  releases: `${ABOUT_REPO_URL}/releases/latest`,
  /** Bug reports and feature requests. */
  issues: `${ABOUT_REPO_URL}/issues`,
  /** Pull requests. */
  pulls: `${ABOUT_REPO_URL}/pulls`,
  /** Full license text. */
  license: `${ABOUT_REPO_URL}/blob/main/LICENSE`,
} as const;

/** Non-localized facts rendered verbatim in the facts list. */
export const ABOUT_FACTS = {
  license: "GPL-3.0-or-later",
  stack: "Tauri 2 · Vue 3 · TypeScript",
} as const;
