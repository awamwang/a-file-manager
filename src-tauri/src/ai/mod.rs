// SPDX-License-Identifier: GPL-3.0-or-later
//! AI Add-on layer (AIA-214 / A1 · AIA-215 / A2 · AIA-216 / A3).
//!
//! **Boundary (ADR 0007):** Workspace / Projects Root / WG / Navigator ∈ Core FM.
//! This module owns the AI Add-on surface: main menu shell, Agent Target registry,
//! public Skill library, and later MCP deploy / Providers / Harness (A4+).
//!
//! - A1: shell status + section ids
//! - A2: Agent Target registry (discover / enable / custom + MCP usage prompt placeholder)
//! - A3: public Skill library + import (SKILL.md required)
//! - Not yet: Skill Junction deploy (A4), MCP catalog/merge (A5), Providers (A7)

mod agents;
mod skills;

pub use agents::{AgentTargetPatch, AiConfigDocument};

use serde::Serialize;
use tauri::AppHandle;
use tauri::Manager;

/// Stable section ids for the AI main menu shell (UI + future IPC).
pub const AI_MENU_SECTION_IDS: &[&str] = &["agents", "mcp", "skills", "providers"];

pub const AI_ADDON_SHELL_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMenuSectionStatus {
    pub id: String,
    /// Agents ready since A2; Skills ready since A3.
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
    matches!(id, "agents" | "skills")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_marks_agents_and_skills_ready() {
        let status = ai_addon_status();
        assert_eq!(status.layer, "ai-addon");
        assert_eq!(status.shell_version, AI_ADDON_SHELL_VERSION);
        for id in ["agents", "skills"] {
            assert!(
                status.sections.iter().find(|s| s.id == id).unwrap().ready,
                "{id} should be ready"
            );
        }
        assert!(status
            .sections
            .iter()
            .filter(|s| s.id != "agents" && s.id != "skills")
            .all(|s| !s.ready));
    }
}
