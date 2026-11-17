// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Shared "Open with" registry client.
 *
 * One backend call (`ide_list_apps`) feeds BOTH the file Navigator context
 * menu and the Project "open with" list, so the two flows stay in sync. Each
 * tool carries project behaviour (`supportsProject` / `projectOpenMode` /
 * `foldersOnly` / `projectArgs`) plus a `builtin` flag separating presets from
 * user-defined custom tools.
 */
import { invoke } from "@tauri-apps/api/core";

export type ProjectOpenMode = "workspace" | "contextFile";

export type OpenToolInfo = {
  id: string;
  name: string;
  enabled: boolean;
  available: boolean;
  resolvedPath: string | null;
  resolution: string;
  configuredPath: string;
  builtin: boolean;
  supportsProject: boolean;
  projectOpenMode: ProjectOpenMode;
  foldersOnly: boolean;
  projectArgs: string[];
};

/** A user-defined custom tool, persisted as JSON in `core.openTool.custom`. */
export type CustomOpenTool = {
  id: string;
  name: string;
  path: string;
  enabled: boolean;
  supportsProject: boolean;
  projectOpenMode: ProjectOpenMode;
  foldersOnly: boolean;
  projectArgs: string[];
};

export const CUSTOM_TOOLS_SETTING_KEY = "core.openTool.custom";

export async function listOpenTools(): Promise<OpenToolInfo[]> {
  return invoke<OpenToolInfo[]>("ide_list_apps");
}

/** Launchers safe to offer for a plain file (folders-only tools excluded). */
export function toolsForFile(tools: OpenToolInfo[]): OpenToolInfo[] {
  return tools.filter((tool) => tool.available && tool.enabled && !tool.foldersOnly);
}

/** Launchers safe to offer for a Project workspace. */
export function toolsForProject(tools: OpenToolInfo[]): OpenToolInfo[] {
  return tools.filter((tool) => tool.available && tool.enabled && tool.supportsProject);
}

/** Parse the persisted custom-tool blob; malformed input degrades to empty. */
export function parseCustomTools(raw: unknown): CustomOpenTool[] {
  if (Array.isArray(raw)) return raw as CustomOpenTool[];
  if (typeof raw !== "string" || raw.trim() === "") return [];
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as CustomOpenTool[]) : [];
  } catch {
    return [];
  }
}
