// SPDX-License-Identifier: GPL-3.0-or-later
/** AI Add-on front-end types (AIA-214 / A1 · AIA-215 / A2 · AIA-263). Core FM must not import these for Workspace ops. */

export type AiMenuSectionId = "providers" | "mcp" | "skills" | "agents" | "harness";

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
  icon: "cloud" | "puzzle" | "link" | "viewAll" | "flash";
};

/** Agent Target registry row (ADR 0006). */
export type AgentTarget = {
  id: string;
  displayName: string;
  kind: "builtin" | "custom" | string;
  enabled: boolean;
  discovered: boolean;
  globalSkillsDir: string;
  projectSkillsDir: string;
  globalMcpPath: string;
  projectMcpPath: string;
  /** Placeholder only — never injected (A2). */
  mcpUsagePrompt: string;
  createdAt: number;
  updatedAt: number;
};

export type McpCatalogEntry = {
  id: string;
  displayName: string;
  serverJson: string;
  createdAt: number;
  updatedAt: number;
};

export type AiProvider = {
  id: string;
  displayName: string;
  kind: "preset" | "compatible" | string;
  enabled: boolean;
  baseUrl: string;
  apiKey: string;
  defaultModel: string;
  createdAt: number;
  updatedAt: number;
};

export type AiConfigDocument = {
  schemaVersion: number;
  agentTargets: AgentTarget[];
  skillsLibraryRoot?: string;
  mcpCatalog?: McpCatalogEntry[];
  aiProviders?: AiProvider[];
};

export type AgentTargetPatch = {
  displayName?: string | null;
  enabled?: boolean | null;
  globalSkillsDir?: string | null;
  projectSkillsDir?: string | null;
  globalMcpPath?: string | null;
  projectMcpPath?: string | null;
  mcpUsagePrompt?: string | null;
};

export type LibrarySkill = {
  name: string;
  path: string;
  manifestPath: string;
  importedAt: number;
};

export type SkillsLibraryStatus = {
  root: string;
  skills: LibrarySkill[];
};

export type SkillDeployReport = {
  skillName: string;
  deployments: Array<{ agentId: string; scope: string; junctionPath: string }>;
  skipped: string[];
};

export type ProjectSkillHit = {
  name: string;
  path: string;
};

export type McpDeployReport = {
  written: string[];
  skipped: string[];
};

export type McpUndeployReport = {
  written: string[];
};

export type McpImportReport = {
  added: string[];
  skipped: string[];
};

export type AiProviderPatch = {
  displayName?: string | null;
  baseUrl?: string | null;
  apiKey?: string | null;
  defaultModel?: string | null;
};

export type HarnessInitReport = {
  workspacePath: string;
  includeSdd: boolean;
  written: string[];
  skipped: string[];
};

export type AiProjectOpenEditorReport = {
  ideId: string;
  openedPath: string;
  mode: string;
  success: boolean;
  error: string | null;
};
