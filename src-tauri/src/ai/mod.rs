// SPDX-License-Identifier: GPL-3.0-or-later
//! AI Add-on layer (AIA-214…AIA-221 / A1–A8).
//!
//! **Boundary (ADR 0007):** Workspace / Projects Root / WG / Navigator ∈ Core FM.
//! This module owns the AI Add-on surface: main menu shell, Agent Target registry,
//! public Skill/MCP libraries, and explicit Harness/SDD init.
//!
//! - A1: shell status + section ids
//! - A2: Agent Target registry (discover / enable / custom + MCP usage prompt placeholder)
//! - A3: public Skill library + import (SKILL.md required)
//! - A4: Skill Junction deploy to enabled Agent Targets
//! - A5: public MCP catalog + merge deploy (no whole-file overwrite)
//! - A6: explicit Harness / SDD init (ADR 0008)
//! - A7: AI Providers MVP (presets + Compatible)
//! - A8: open Project Workspace in IDE (Cursor dir / Qoder CONTEXT.md)

mod agents;
mod deploy;
mod harness;
mod mcp;
mod open_editor;
mod providers;
mod skills;

pub use agents::{AgentTargetPatch, AiConfigDocument};
pub use harness::HarnessInitReport;
pub use open_editor::OpenEditorReport;
pub use providers::AiProviderPatch;

use serde::Serialize;
use tauri::AppHandle;
use tauri::Manager;

/// Stable section ids for the AI main menu shell (UI + future IPC).
pub const AI_MENU_SECTION_IDS: &[&str] = &["agents", "mcp", "skills", "providers"];

pub const AI_ADDON_SHELL_VERSION: u32 = 6;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMenuSectionStatus {
    pub id: String,
    /// Agents A2 · Skills A3 · MCP A5 · Providers A7.
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiAddonStatus {
    /// Always `"ai-addon"` — distinguishes from Core FM modules.
    pub layer: String,
    pub shell_version: u32,
    pub sections: Vec<AiMenuSectionStatus>,
}

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

fn section_ready(id: &str) -> bool {
    matches!(id, "agents" | "skills" | "mcp" | "providers")
}

/// Shell probe for the AI main menu.
#[tauri::command]
pub fn ai_addon_status() -> AiAddonStatus {
    AiAddonStatus {
        layer: "ai-addon".to_string(),
        shell_version: AI_ADDON_SHELL_VERSION,
        sections: AI_MENU_SECTION_IDS
            .iter()
            .map(|id| AiMenuSectionStatus {
                id: (*id).to_string(),
                ready: section_ready(id),
            })
            .collect(),
    }
}

/// List Agent Targets (builtins synced + discovery refreshed). Stored in APPDATA `ai-config.json`.
#[tauri::command]
pub fn ai_agent_targets_list(app: AppHandle) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    agents::load_from_path(&dir)
}

/// Re-run discovery against current path fields (same as list sync).
#[tauri::command]
pub fn ai_agent_targets_discover(app: AppHandle) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let current = agents::load_from_path(&dir)?;
    let next = agents::sync_builtins_and_discover(&current);
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_agent_targets_set_enabled(
    app: AppHandle,
    id: String,
    enabled: bool,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let current = agents::load_from_path(&dir)?;
    let next = agents::set_enabled(&current, &id, enabled)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_agent_targets_update(
    app: AppHandle,
    id: String,
    patch: AgentTargetPatch,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let current = agents::load_from_path(&dir)?;
    let next = agents::update_target(&current, &id, patch)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

/// Create or update a **custom** Agent Target (cannot overwrite builtin ids).
#[tauri::command]
pub fn ai_agent_targets_upsert_custom(
    app: AppHandle,
    id: Option<String>,
    display_name: String,
    global_skills_dir: String,
    project_skills_dir: String,
    global_mcp_path: String,
    project_mcp_path: String,
    mcp_usage_prompt: String,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let current = agents::load_from_path(&dir)?;
    let next = agents::upsert_custom(
        &current,
        id,
        display_name,
        global_skills_dir,
        project_skills_dir,
        global_mcp_path,
        project_mcp_path,
        mcp_usage_prompt,
    )?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_agent_targets_remove(
    app: AppHandle,
    id: String,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let current = agents::load_from_path(&dir)?;
    let next = agents::remove_custom(&current, &id)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

/// Public Skill library status (root + imported skills with SKILL.md).
#[tauri::command]
pub fn ai_skills_library_get(app: AppHandle) -> Result<skills::SkillsLibraryStatus, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    skills::library_status(&dir, &doc)
}

/// Set or clear skills library root override (empty / null → default under APPDATA).
#[tauri::command]
pub fn ai_skills_library_set_root(
    app: AppHandle,
    root: Option<String>,
) -> Result<skills::SkillsLibraryStatus, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let (next, status) = skills::set_library_root(&dir, &doc, root)?;
    skills::save_ai_config(&dir, &next)?;
    Ok(status)
}

/// Import a folder into the public Skill library. Root must contain `SKILL.md`.
#[tauri::command]
pub fn ai_skills_import(
    app: AppHandle,
    source_path: String,
    name: Option<String>,
) -> Result<(skills::SkillsLibraryStatus, skills::LibrarySkill), String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    skills::import_skill(&dir, &doc, &source_path, name)
}

#[tauri::command]
pub fn ai_skills_remove(app: AppHandle, name: String) -> Result<skills::SkillsLibraryStatus, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    skills::remove_skill(&dir, &doc, &name)
}

/// Deploy a library skill via Junction into all **enabled** Agent Target skills dirs.
/// Optional `projectWorkspace` also writes project-relative junctions under that Workspace.
#[tauri::command]
pub fn ai_skills_deploy(
    app: AppHandle,
    skill_name: String,
    project_workspace: Option<String>,
) -> Result<deploy::DeployReport, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    deploy::deploy_skill(
        &dir,
        &doc,
        &skill_name,
        project_workspace.as_deref(),
    )
}

#[tauri::command]
pub fn ai_skills_undeploy(
    app: AppHandle,
    skill_name: String,
    project_workspace: Option<String>,
) -> Result<deploy::DeployReport, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    deploy::undeploy_skill(
        &dir,
        &doc,
        &skill_name,
        project_workspace.as_deref(),
    )
}

/// Search a Project Workspace for folders containing `SKILL.md` (for add-to-library).
#[tauri::command]
pub fn ai_skills_search_project(
    workspace_path: String,
) -> Result<Vec<deploy::ProjectSkillCandidate>, String> {
    deploy::search_project_skills(&workspace_path)
}

#[tauri::command]
pub fn ai_mcp_catalog_list(app: AppHandle) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    agents::load_from_path(&dir)
}

#[tauri::command]
pub fn ai_mcp_catalog_upsert(
    app: AppHandle,
    id: Option<String>,
    display_name: String,
    server_json: String,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let next = mcp::upsert_entry(&doc, id, display_name, server_json)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_mcp_catalog_remove(app: AppHandle, id: String) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let next = mcp::remove_entry(&doc, &id)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

/// Merge-deploy one catalog entry into enabled Agent MCP configs (managed `afm__*` keys only).
#[tauri::command]
pub fn ai_mcp_deploy(
    app: AppHandle,
    entry_id: String,
    project_workspace: Option<String>,
) -> Result<mcp::McpMergeReport, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    mcp::deploy_entry(&doc, &entry_id, project_workspace.as_deref())
}

#[tauri::command]
pub fn ai_mcp_undeploy(
    app: AppHandle,
    entry_id: String,
    project_workspace: Option<String>,
) -> Result<mcp::McpMergeReport, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    mcp::undeploy_entry(&doc, &entry_id, project_workspace.as_deref())
}

/// Explicit Harness / Superpower init on a Project Workspace (ADR 0008).
/// `include_sdd` defaults to false. Does not run from Core Project create.
#[tauri::command]
pub fn ai_harness_init(
    workspace_path: String,
    include_sdd: Option<bool>,
    reinforce_context: Option<bool>,
) -> Result<HarnessInitReport, String> {
    harness::init_harness(
        &workspace_path,
        include_sdd.unwrap_or(false),
        reinforce_context.unwrap_or(true),
    )
}

#[tauri::command]
pub fn ai_providers_list(app: AppHandle) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    agents::load_from_path(&dir)
}

#[tauri::command]
pub fn ai_providers_set_enabled(
    app: AppHandle,
    id: String,
    enabled: bool,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let next = providers::set_enabled(&doc, &id, enabled)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_providers_update(
    app: AppHandle,
    id: String,
    patch: AiProviderPatch,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let next = providers::update_provider(&doc, &id, patch)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_providers_upsert_compatible(
    app: AppHandle,
    id: Option<String>,
    display_name: String,
    base_url: String,
    api_key: String,
    default_model: String,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let next = providers::upsert_compatible(
        &doc,
        id,
        display_name,
        base_url,
        api_key,
        default_model,
    )?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_providers_remove(app: AppHandle, id: String) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let next = providers::remove_compatible(&doc, &id)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

/// Open a Project Workspace in Cursor (dir) / Qoder (`CONTEXT.md`) etc. (A8).
#[tauri::command]
pub fn ai_project_open_editor(
    app: AppHandle,
    workspace_path: String,
    ide_id: Option<String>,
) -> Result<OpenEditorReport, String> {
    open_editor::open_project_in_editor(&app, &workspace_path, ide_id.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_marks_all_sections_ready() {
        let status = ai_addon_status();
        assert_eq!(status.layer, "ai-addon");
        assert_eq!(status.shell_version, AI_ADDON_SHELL_VERSION);
        for id in ["agents", "skills", "mcp", "providers"] {
            assert!(
                status.sections.iter().find(|s| s.id == id).unwrap().ready,
                "{id} should be ready"
            );
        }
    }
}
