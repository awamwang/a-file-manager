// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AI Add-on front-end entry (AIA-214 / A1 · AIA-215 / A2).
 *
 * Import AI UI/state from here — not from Core FM modules (`projects`, `work_groups`,
 * Navigator). Keeps the Add-on boundary discoverable for A3+.
 */
export { AI_MENU_SECTION_IDS, AI_MENU_SECTIONS } from "./sections";
export type {
  AgentTarget,
  AgentTargetPatch,
  AiAddonStatus,
  AiConfigDocument,
  AiMenuSectionDef,
  AiMenuSectionId,
  AiMenuSectionStatus,
  AiProvider,
  McpCatalogEntry,
} from "./types";
