// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * The single declarative guidance registry (ADR 0021).
 *
 * Every layer reads from here: the Help Center lists entries, hints resolve
 * their copy key, and tours resolve their steps. New features must register one
 * entry instead of hard-coding help UI somewhere else.
 *
 * i18n keys in this file are intentionally string literals so that
 * `scripts/check-i18n.mjs` can verify five-locale coverage without a TS runtime.
 */

import { AI_ADDON_ENABLED } from "../config/flavor.ts";
import { isMacPlatform } from "../config/platform.ts";
import type {
  Applicability,
  ApplicabilityContext,
  HelpCategory,
  HelpEntry,
  TourDefinition,
  TourStep,
} from "./types.ts";

/** Display order of categories in the Help Center. */
export const HELP_CATEGORIES: readonly HelpCategory[] = [
  "browse",
  "organize",
  "connect",
  "ai",
];

/** i18n key for a category label. */
export function categoryKey(category: HelpCategory): string {
  return `help.category.${category}`;
}

/** First-run main line: browse files → search → collect/favourite places. */
export const QUICK_START_TOUR_ID = "quickstart";

const QUICK_START_STEPS: readonly TourStep[] = [
  {
    anchor: "tour-navigator-list",
    titleKey: "tour.quickStart.browse.title",
    bodyKey: "tour.quickStart.browse.body",
  },
  {
    anchor: "tour-global-search",
    titleKey: "tour.quickStart.search.title",
    bodyKey: "tour.quickStart.search.body",
  },
  {
    anchor: "tour-quick-tags",
    titleKey: "tour.quickStart.tags.title",
    bodyKey: "tour.quickStart.tags.body",
  },
];

const ALL_ENTRIES: readonly HelpEntry[] = [
  {
    id: "navigator",
    titleKey: "help.entries.navigator.title",
    category: "browse",
    route: "/navigator",
    articlePath: "./content/navigator.md",
    hintKey: "help.entries.navigator.hint",
    tourId: "navigator",
    applicability: { routes: ["/navigator", "/"] },
  },
  {
    id: "search",
    titleKey: "help.entries.search.title",
    category: "browse",
    route: "/settings/discovery",
    articlePath: "./content/search.md",
    hintKey: "help.entries.search.hint",
    tourId: "search",
    applicability: { routes: ["/settings/discovery", "/navigator"] },
  },
  {
    id: "work-groups",
    titleKey: "help.entries.workGroups.title",
    category: "organize",
    route: "/work-groups",
    articlePath: "./content/work-groups.md",
    hintKey: "help.entries.workGroups.hint",
    tourId: "work-groups",
    applicability: { routes: ["/work-groups"] },
  },
  {
    id: "projects",
    titleKey: "help.entries.projects.title",
    category: "organize",
    route: "/projects",
    articlePath: "./content/projects.md",
    hintKey: "help.entries.projects.hint",
    tourId: "projects",
    applicability: { routes: ["/projects"] },
  },
  {
    id: "providers",
    titleKey: "help.entries.providers.title",
    category: "connect",
    route: "/providers",
    articlePath: "./content/providers.md",
    hintKey: "help.entries.providers.hint",
    tourId: "providers",
    applicability: { routes: ["/providers"] },
  },
  {
    id: "ai",
    titleKey: "help.entries.ai.title",
    category: "ai",
    route: "/ai",
    articlePath: "./content/ai.md",
    hintKey: "help.entries.ai.hint",
    tourId: "ai",
    applicability: { routes: ["/ai"], featureFlag: "ai" },
  },
];

const ALL_TOURS: readonly TourDefinition[] = [
  {
    id: QUICK_START_TOUR_ID,
    titleKey: "help.quickStartTitle",
    steps: QUICK_START_STEPS,
  },
  {
    id: "navigator",
    titleKey: "help.entries.navigator.title",
    steps: [
      {
        anchor: "tour-navigator-list",
        titleKey: "tour.navigator.list.title",
        bodyKey: "tour.navigator.list.body",
      },
      {
        anchor: "tour-navigator-path",
        titleKey: "tour.navigator.path.title",
        bodyKey: "tour.navigator.path.body",
      },
      {
        anchor: "tour-navigator-help",
        titleKey: "tour.navigator.help.title",
        bodyKey: "tour.navigator.help.body",
        requireClick: true,
      },
    ],
  },
  {
    id: "search",
    titleKey: "help.entries.search.title",
    steps: [
      {
        anchor: "tour-global-search",
        titleKey: "tour.search.open.title",
        bodyKey: "tour.search.open.body",
      },
      {
        anchor: "tour-search-input",
        titleKey: "tour.search.input.title",
        bodyKey: "tour.search.input.body",
        requireClick: true,
      },
      {
        anchor: "tour-search-scopes",
        titleKey: "tour.search.scopes.title",
        bodyKey: "tour.search.scopes.body",
      },
    ],
  },
  {
    id: "work-groups",
    titleKey: "help.entries.workGroups.title",
    steps: [
      {
        anchor: "tour-workgroups-header",
        titleKey: "tour.workGroups.header.title",
        bodyKey: "tour.workGroups.header.body",
      },
      {
        anchor: "tour-workgroups-list",
        titleKey: "tour.workGroups.list.title",
        bodyKey: "tour.workGroups.list.body",
      },
      {
        anchor: "tour-workgroups-group",
        titleKey: "tour.workGroups.group.title",
        bodyKey: "tour.workGroups.group.body",
        requireClick: true,
      },
    ],
  },
  {
    id: "projects",
    titleKey: "help.entries.projects.title",
    steps: [
      {
        anchor: "tour-projects-header",
        titleKey: "tour.projects.header.title",
        bodyKey: "tour.projects.header.body",
      },
      {
        anchor: "tour-projects-list",
        titleKey: "tour.projects.list.title",
        bodyKey: "tour.projects.list.body",
      },
      {
        anchor: "tour-projects-group",
        titleKey: "tour.projects.group.title",
        bodyKey: "tour.projects.group.body",
        requireClick: true,
      },
    ],
  },
  {
    id: "providers",
    titleKey: "help.entries.providers.title",
    steps: [
      {
        anchor: "tour-providers-gateway",
        titleKey: "tour.providers.gateway.title",
        bodyKey: "tour.providers.gateway.body",
      },
      {
        anchor: "tour-providers-backends",
        titleKey: "tour.providers.backends.title",
        bodyKey: "tour.providers.backends.body",
      },
      {
        anchor: "tour-providers-refresh",
        titleKey: "tour.providers.refresh.title",
        bodyKey: "tour.providers.refresh.body",
        requireClick: true,
      },
    ],
  },
  {
    id: "ai",
    titleKey: "help.entries.ai.title",
    steps: [
      {
        anchor: "tour-ai-header",
        titleKey: "tour.ai.header.title",
        bodyKey: "tour.ai.header.body",
      },
      {
        anchor: "tour-ai-nav",
        titleKey: "tour.ai.nav.title",
        bodyKey: "tour.ai.nav.body",
      },
      {
        anchor: "tour-ai-panel",
        titleKey: "tour.ai.panel.title",
        bodyKey: "tour.ai.panel.body",
      },
    ],
  },
];

/** True when an entry's conditions are satisfied in the given context. */
export function isApplicable(
  applicability: Applicability | undefined,
  context: ApplicabilityContext,
): boolean {
  if (!applicability) return true;
  if (applicability.featureFlag === "ai" && !context.aiEnabled) return false;
  if (applicability.platform && applicability.platform !== (context.mac ? "macos" : "windows")) {
    return false;
  }
  if (applicability.routes?.length) {
    const onRoute = applicability.routes.some(
      (route) => context.path === route || context.path.startsWith(`${route}/`),
    );
    const helpRoute = context.path === "/help" || context.path.startsWith("/help/");
    if (!onRoute && !helpRoute) return false;
  }
  return true;
}

/** Default context for the current runtime (build flavor + host platform). */
export function defaultContext(path: string = window.location.pathname): ApplicabilityContext {
  return { path, aiEnabled: AI_ADDON_ENABLED, mac: isMacPlatform() };
}

/** Registry filtered by build flavor / platform gates (route gates are per-surface). */
export const helpRegistry: readonly HelpEntry[] = ALL_ENTRIES.filter((entry) =>
  isApplicable(entry.applicability, defaultContext()),
);

/** All tours whose gates are satisfied in this build. */
export const tourRegistry: Readonly<Record<string, TourDefinition>> = Object.fromEntries(
  ALL_TOURS.filter(
    (tour) =>
      tour.id === QUICK_START_TOUR_ID ||
      helpRegistry.some((entry) => entry.tourId === tour.id),
  ).map((tour) => [tour.id, tour]),
);

/** Look up one entry by id; returns undefined when the build does not include it. */
export function getHelpEntry(id: string): HelpEntry | undefined {
  return helpRegistry.find((entry) => entry.id === id);
}

/** Entries relevant to a route (Help Center always shows every registered entry). */
export function applicableHelpEntries(path: string): readonly HelpEntry[] {
  const context = defaultContext(path);
  return helpRegistry.filter((entry) => isApplicable(entry.applicability, context));
}

/** Tour steps for an entry id, or undefined when the entry has no tour. */
export function tourForEntry(entry: HelpEntry | undefined): TourDefinition | undefined {
  return entry?.tourId ? tourRegistry[entry.tourId] : undefined;
}

/** Build-time Markdown lookup (Vite `?raw`, no IPC / no Rust changes). */
const ARTICLE_MODULES = import.meta.glob("./content/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** Return the embedded Markdown for an article path, or an empty string. */
export function articleMarkdown(articlePath: string): string {
  return ARTICLE_MODULES[articlePath] ?? "";
}
