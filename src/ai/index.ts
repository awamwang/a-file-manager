// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AI Add-on front-end entry (AIA-214 / A1 · AIA-215 / A2).
 *
 * Import AI UI/state from here — not from Core FM modules (`projects`, `work_groups`,
 * Navigator). Keeps the Add-on boundary discoverable for A3+.
 */
export { AI_MENU_SECTION_IDS, AI_MENU_SECTIONS } from "./sections";
export {
  aiAddonStatus,
  aiAgentTargetsDiscover,
  aiAgentTargetsList,
  aiAgentTargetsRemove,
  aiAgentTargetsSetEnabled,
  aiAgentTargetsUpdate,
  aiAgentTargetsUpsertCustom,
  aiHarnessInit,
  aiMcpCatalogImport,
  aiMcpCatalogList,
  aiMcpCatalogRemove,
  aiMcpCatalogUpsert,
  aiMcpDeploy,
  aiMcpUndeploy,
  aiProjectOpenEditor,
  aiProvidersList,
  aiProvidersRemove,
  aiProvidersSetEnabled,
  aiProvidersUpdate,
  aiProvidersUpsertCompatible,
  aiSkillsDeploy,
  aiSkillsImport,
  aiSkillsLibraryGet,
  aiSkillsLibrarySetRoot,
  aiSkillsRemove,
  aiSkillsSearchProject,
  aiSkillsUndeploy,
} from "./client";
export type {
  AgentTarget,
  AgentTargetPatch,
  AiAddonStatus,
  AiConfigDocument,
  AiMenuSectionDef,
  AiMenuSectionId,
  AiMenuSectionStatus,
  AiProjectOpenEditorReport,
  AiProvider,
  AiProviderPatch,
  HarnessInitReport,
  LibrarySkill,
  McpCatalogEntry,
  McpDeployReport,
  McpImportReport,
  McpUndeployReport,
  ProjectSkillHit,
  SkillDeployReport,
  SkillsLibraryStatus,
} from "./types";
