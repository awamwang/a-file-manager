// SPDX-License-Identifier: GPL-3.0-or-later
/** AI Add-on front-end types (AIA-214 / A1). Core FM must not import these for Workspace ops. */

export type AiMenuSectionId = "agents" | "mcp" | "skills" | "providers";

export type AiMenuSectionStatus = {
  id: AiMenuSectionId | string;
  ready: boolean;
};

export type AiAddonStatus = {
  layer: "ai-addon" | string;
  shellVersion: number;
  sections: AiMenuSectionStatus[];
};

export type AiMenuSectionDef = {
  id: AiMenuSectionId;
  titleKey: string;
  bodyKey: string;
  /** Glyph for the section tile (Fluent catalog id). */
  icon: "viewAll" | "puzzle" | "link" | "cloud";
};
