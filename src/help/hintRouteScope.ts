// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Pure route scope for L2 getting-started hints (ADR 0021 / ADR 0025).
 *
 * A hint is pinned to the functional page it describes, never to the whole app
 * shell. This matcher is deliberately free of Vue / registry imports so the
 * regression gate can execute the very same logic in plain Node: a hint whose
 * page is no longer the active route must stop being visible.
 */

/** The registry fields a hint needs to resolve its page binding. */
export type HintPageBinding = {
  route?: string;
  applicability?: { routes?: readonly string[] };
};

/** Route prefixes a hint is bound to, in registry order; empty means no page. */
export function routesForHintEntry(entry: HintPageBinding | undefined): readonly string[] {
  if (!entry) return [];
  const declared = entry.applicability?.routes;
  if (declared?.length) return declared;
  return entry.route ? [entry.route] : [];
}

/**
 * Exact-or-child path match. A sibling prefix (`/work-groups-extra`) or an
 * unrelated surface (`/help`) is never treated as "on this page".
 */
export function isHintPathOnRoutes(routes: readonly string[], path: string): boolean {
  if (!path) return false;
  return routes.some((route) =>
    Boolean(route) && (path === route || path.startsWith(`${route}/`)),
  );
}

/** True only while the hint's own functional page is the active route. */
export function isHintPageActive(entry: HintPageBinding | undefined, path: string): boolean {
  return isHintPathOnRoutes(routesForHintEntry(entry), path);
}
