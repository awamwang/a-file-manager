// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AI main-menu section model (AIA-214 / A1).
 * Order matches Rust `AI_MENU_SECTION_IDS` — keep in sync.
 */
import type { AiMenuSectionDef } from "./types";

export const AI_MENU_SECTIONS: readonly AiMenuSectionDef[] = [
  {
    id: "agents",
    titleKey: "ai.sections.agents.title",
    bodyKey: "ai.sections.agents.body",
    icon: "viewAll",
  },
  {
    id: "mcp",
    titleKey: "ai.sections.mcp.title",
    bodyKey: "ai.sections.mcp.body",
    icon: "puzzle",
  },
  {
    id: "skills",
    titleKey: "ai.sections.skills.title",
    bodyKey: "ai.sections.skills.body",
    icon: "link",
  },
  {
    id: "providers",
    titleKey: "ai.sections.providers.title",
    bodyKey: "ai.sections.providers.body",
    icon: "cloud",
  },
] as const;

export const AI_MENU_SECTION_IDS = AI_MENU_SECTIONS.map((s) => s.id);
