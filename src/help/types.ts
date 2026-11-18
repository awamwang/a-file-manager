// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Type contracts for the in-app help & guidance system (ADR 0021).
 *
 * Three layers share these definitions: help articles (L1), getting-started
 * hints (L2) and guided tours (L3). A new feature joins the system by adding
 * one HelpEntry (plus, optionally, one TourDefinition) to the registry.
 */

/** Coarse grouping used by the Help Center list. */
export type HelpCategory = "browse" | "organize" | "connect" | "ai";

/** Build-flavor gates understood by the registry. */
export type FeatureFlag = "ai";

/** Platform gates understood by the registry. */
export type PlatformId = "windows" | "macos";

/**
 * Conditions under which a registry record is meaningful. All fields are
 * optional: an entry without conditions is always available.
 */
export type Applicability = {
  /** Route prefixes where the entry is relevant (used for deep links/hints). */
  routes?: readonly string[];
  /** Build flavor gate; `ai` maps to AI_ADDON_ENABLED in src/config/flavor.ts. */
  featureFlag?: FeatureFlag;
  /** Platform gate; omit for both. */
  platform?: PlatformId;
};

/** One spotlight step. `anchor` is a stable `data-tour-id` attribute value. */
export type TourStep = {
  anchor: string;
  titleKey: string;
  bodyKey: string;
  /** When true the user must click the anchor to advance. */
  requireClick?: boolean;
};

/** A named, replayable sequence of spotlight steps. */
export type TourDefinition = {
  id: string;
  titleKey: string;
  steps: readonly TourStep[];
};

/** One row of the declarative guidance registry. */
export type HelpEntry = {
  id: string;
  titleKey: string;
  category: HelpCategory;
  route: string;
  /** Vite `?raw` module path under src/help/content/. */
  articlePath: string;
  hintKey?: string;
  tourId?: string;
  applicability?: Applicability;
};

/** Context used to decide whether an entry applies to the current surface. */
export type ApplicabilityContext = {
  path: string;
  aiEnabled: boolean;
  mac: boolean;
};
