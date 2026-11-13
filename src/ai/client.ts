// SPDX-License-Identifier: GPL-3.0-or-later
/** AI Add-on 共享 typed invoke 客户端 — 面板与 Core 调用方经此访问 IPC。 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AgentTargetPatch,
  AiAddonStatus,
  AiConfigDocument,
  AiProjectDocument,
  AiProjectOpenEditorReport,
  AiProviderPatch,
  HarnessInitReport,
  HarvestMcpCandidate,
  HarvestMcpScanReport,
  HarvestSkillCandidate,
  HarvestSkillScanReport,
  LibrarySkill,
  McpDeployReport,
  McpImportReport,
  McpUndeployReport,
  ProjectSkillHit,
  SkillDeployReport,
  SkillsLibraryStatus,
} from "./types.ts";

export function aiAddonStatus(): Promise<AiAddonStatus> {
  return invoke<AiAddonStatus>("ai_addon_status");
}

export function aiAgentTargetsList(): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_agent_targets_list");
}

export function aiAgentTargetsDiscover(): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_agent_targets_discover");
}

export function aiAgentTargetsSetEnabled(
  id: string,
  enabled: boolean,
): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_agent_targets_set_enabled", { id, enabled });
}

export function aiAgentTargetsSetDefaultInProjects(
  id: string,
  defaultInProjects: boolean,
): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_agent_targets_set_default_in_projects", {
    id,
    defaultInProjects,
  });
}

export function aiAgentTargetsUpdate(
  id: string,
  patch: AgentTargetPatch,
): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_agent_targets_update", { id, patch });
}

export function aiAgentTargetsUpsertCustom(args: {
  id: string | null;
  displayName: string;
  globalSkillsDir: string;
  projectSkillsDir: string;
  globalMcpPath: string;
  projectMcpPath: string;
  mcpUsagePrompt: string;
}): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_agent_targets_upsert_custom", args);
}

export function aiAgentTargetsRemove(id: string): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_agent_targets_remove", { id });
}

export function aiSkillDeployModeSet(mode: string): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_skill_deploy_mode_set", { mode });
}

export function aiSkillsLibraryGet(): Promise<SkillsLibraryStatus> {
  return invoke<SkillsLibraryStatus>("ai_skills_library_get");
}

export function aiSkillsLibrarySetRoot(root: string | null): Promise<SkillsLibraryStatus> {
  return invoke<SkillsLibraryStatus>("ai_skills_library_set_root", { root });
}

export function aiSkillsImport(
  sourcePath: string,
  name: string | null,
): Promise<[SkillsLibraryStatus, LibrarySkill]> {
  return invoke<[SkillsLibraryStatus, LibrarySkill]>("ai_skills_import", {
    sourcePath,
    name,
  });
}

export function aiSkillsRemove(name: string): Promise<SkillsLibraryStatus> {
  return invoke<SkillsLibraryStatus>("ai_skills_remove", { name });
}

export function aiSkillsDeploy(
  skillName: string,
  projectWorkspace: string | null,
): Promise<SkillDeployReport> {
  return invoke<SkillDeployReport>("ai_skills_deploy", { skillName, projectWorkspace });
}

export function aiSkillsUndeploy(
  skillName: string,
  projectWorkspace: string | null,
): Promise<SkillDeployReport> {
  return invoke<SkillDeployReport>("ai_skills_undeploy", { skillName, projectWorkspace });
}

export function aiSkillsSearchProject(workspacePath: string): Promise<ProjectSkillHit[]> {
  return invoke<ProjectSkillHit[]>("ai_skills_search_project", { workspacePath });
}

export function aiMcpCatalogList(): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_mcp_catalog_list");
}

export function aiMcpCatalogUpsert(args: {
  id: string | null;
  displayName: string;
  serverJson: string;
}): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_mcp_catalog_upsert", args);
}

export function aiMcpCatalogRemove(id: string): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_mcp_catalog_remove", { id });
}

export function aiMcpCatalogImport(
  path: string,
): Promise<[AiConfigDocument, McpImportReport]> {
  return invoke<[AiConfigDocument, McpImportReport]>("ai_mcp_catalog_import", { path });
}

export function aiMcpDeploy(
  entryId: string,
  projectWorkspace: string | null,
): Promise<McpDeployReport> {
  return invoke<McpDeployReport>("ai_mcp_deploy", { entryId, projectWorkspace });
}

export function aiMcpUndeploy(
  entryId: string,
  projectWorkspace: string | null,
): Promise<McpUndeployReport> {
  return invoke<McpUndeployReport>("ai_mcp_undeploy", { entryId, projectWorkspace });
}

export function aiHarvestMcpScan(agentId: string): Promise<HarvestMcpScanReport> {
  return invoke<HarvestMcpScanReport>("ai_harvest_mcp_scan", { agentId });
}

export function aiHarvestMcpImport(
  candidates: HarvestMcpCandidate[],
): Promise<[AiConfigDocument, McpImportReport]> {
  return invoke<[AiConfigDocument, McpImportReport]>("ai_harvest_mcp_import", { candidates });
}

export function aiHarvestSkillsScan(agentId: string): Promise<HarvestSkillScanReport> {
  return invoke<HarvestSkillScanReport>("ai_harvest_skills_scan", { agentId });
}

export function aiHarvestSkillsImport(
  candidates: HarvestSkillCandidate[],
): Promise<[SkillsLibraryStatus, string[], string[]]> {
  return invoke<[SkillsLibraryStatus, string[], string[]]>("ai_harvest_skills_import", {
    candidates,
  });
}

export function aiProjectAiGet(workspacePath: string): Promise<AiProjectDocument> {
  return invoke<AiProjectDocument>("ai_project_ai_get", { workspacePath });
}

export function aiProjectAiSetEnabledAgents(
  workspacePath: string,
  enabledAgentIds: string[],
): Promise<AiProjectDocument> {
  return invoke<AiProjectDocument>("ai_project_ai_set_enabled_agents", {
    workspacePath,
    enabledAgentIds,
  });
}

export function aiProvidersList(): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_providers_list");
}

export function aiProvidersSetEnabled(
  id: string,
  enabled: boolean,
): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_providers_set_enabled", { id, enabled });
}

export function aiProvidersUpdate(
  id: string,
  patch: AiProviderPatch,
): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_providers_update", { id, patch });
}

export function aiProvidersUpsertCompatible(args: {
  id: string | null;
  displayName: string;
  baseUrl: string;
  apiKey: string;
  defaultModel: string;
}): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_providers_upsert_compatible", args);
}

export function aiProvidersRemove(id: string): Promise<AiConfigDocument> {
  return invoke<AiConfigDocument>("ai_providers_remove", { id });
}

export function aiHarnessInit(args: {
  workspacePath: string;
  includeSdd: boolean;
  reinforceContext: boolean;
}): Promise<HarnessInitReport> {
  return invoke<HarnessInitReport>("ai_harness_init", args);
}

export function aiProjectOpenEditor(
  workspacePath: string,
  ideId: "cursor" | "qoder" | string,
): Promise<AiProjectOpenEditorReport> {
  return invoke<AiProjectOpenEditorReport>("ai_project_open_editor", {
    workspacePath,
    ideId,
  });
}
