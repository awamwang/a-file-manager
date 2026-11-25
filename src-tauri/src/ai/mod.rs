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
mod agent_paths;
mod deploy;
mod harness;
mod harvest;
mod mcp;
mod open_editor;
mod project_ai;
mod providers;
mod skills;

pub use agents::{AgentTargetPatch, AiConfigDocument};
pub use harness::HarnessInitReport;
pub use open_editor::OpenEditorReport;
pub use providers::AiProviderPatch;

use serde::Serialize;
use std::path::Path;
use tauri::AppHandle;
use tauri::Manager;

/// Stable section ids for the AI main menu shell (UI + future IPC).
/// Order: Providers → Agents → MCP → Skills → Harness (AIA-263 left nav).
pub const AI_MENU_SECTION_IDS: &[&str] = &["providers", "agents", "mcp", "skills", "harness"];

pub const AI_ADDON_SHELL_VERSION: u32 = 9;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMenuSectionStatus {
    pub id: String,
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
pub fn ai_agent_targets_set_default_in_projects(
    app: AppHandle,
    id: String,
    default_in_projects: bool,
) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let current = agents::load_from_path(&dir)?;
    let next = agents::set_default_in_projects(&current, &id, default_in_projects)?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_skill_deploy_mode_set(app: AppHandle, mode: String) -> Result<AiConfigDocument, String> {
    let dir = config_dir(&app)?;
    let current = agents::load_from_path(&dir)?;
    let next = agents::set_skill_deploy_mode(&current, &mode)?;
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
    global_sessions_dir: String,
    global_rules_path: String,
    project_rules_dir: String,
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
        global_sessions_dir,
        global_rules_path,
        project_rules_dir,
        mcp_usage_prompt,
    )?;
    agents::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn ai_agent_targets_remove(app: AppHandle, id: String) -> Result<AiConfigDocument, String> {
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
/// 递归拷贝可能较重，放到阻塞线程，避免拖住后续打开工作区等 IPC（FIL-30）。
#[tauri::command]
pub async fn ai_skills_import(
    app: AppHandle,
    source_path: String,
    name: Option<String>,
) -> Result<(skills::SkillsLibraryStatus, skills::LibrarySkill), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = config_dir(&app)?;
        let doc = agents::load_from_path(&dir)?;
        skills::import_skill(&dir, &doc, &source_path, name)
    })
    .await
    .map_err(|err| format!("ai_skills_import worker failed: {err}"))?
}

#[tauri::command]
pub fn ai_skills_remove(
    app: AppHandle,
    name: String,
) -> Result<skills::SkillsLibraryStatus, String> {
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
    deploy::deploy_skill(&dir, &doc, &skill_name, project_workspace.as_deref())
}

#[tauri::command]
pub fn ai_skills_undeploy(
    app: AppHandle,
    skill_name: String,
    project_workspace: Option<String>,
) -> Result<deploy::DeployReport, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    deploy::undeploy_skill(&dir, &doc, &skill_name, project_workspace.as_deref())
}

/// Search a Project Workspace for folders containing `SKILL.md` (for add-to-library).
/// 项目内发现受 Agent Target 配置驱动：workspace 根浅层 + 项目启用集各 Target 的
/// `projectSkillsDir` 真实配置（ADR 0010 修订：Agent Target 是源头）。
#[tauri::command]
pub fn ai_skills_search_project(
    app: AppHandle,
    workspace_path: String,
) -> Result<Vec<deploy::ProjectSkillCandidate>, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    deploy::search_project_skills(&workspace_path, &doc)
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

/// One-shot import of Cursor/VS Code-style `{ "mcpServers": … }` into the public catalog.
/// Existing ids are skipped (idempotent; does not wipe user edits).
#[tauri::command]
pub fn ai_mcp_catalog_import(
    app: AppHandle,
    path: String,
) -> Result<(AiConfigDocument, mcp::McpImportReport), String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let (next, report) = mcp::import_from_mcp_json_path(&doc, Path::new(&path))?;
    agents::save_to_path(&dir, &next)?;
    Ok((next, report))
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

#[tauri::command]
pub fn ai_harvest_mcp_scan(
    app: AppHandle,
    agent_id: String,
) -> Result<harvest::HarvestMcpScanReport, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    harvest::scan_mcp_from_agent(&doc, &agent_id)
}

#[tauri::command]
pub fn ai_harvest_mcp_import(
    app: AppHandle,
    candidates: Vec<harvest::HarvestMcpCandidate>,
) -> Result<(AiConfigDocument, mcp::McpImportReport), String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let (next, report) = harvest::import_mcp_candidates(&doc, &candidates)?;
    agents::save_to_path(&dir, &next)?;
    Ok((next, report))
}

#[tauri::command]
pub fn ai_harvest_skills_scan(
    app: AppHandle,
    agent_id: String,
) -> Result<harvest::HarvestSkillScanReport, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    harvest::scan_skills_from_agent(&dir, &doc, &agent_id)
}

#[tauri::command]
pub fn ai_harvest_skills_import(
    app: AppHandle,
    candidates: Vec<harvest::HarvestSkillCandidate>,
) -> Result<(skills::SkillsLibraryStatus, Vec<String>, Vec<String>), String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let (status, imported, skipped) = harvest::import_skill_candidates(&dir, &doc, &candidates)?;
    Ok((
        status,
        imported.into_iter().map(|s| s.name).collect(),
        skipped,
    ))
}

#[tauri::command]
pub fn ai_project_ai_get(
    app: AppHandle,
    workspace_path: String,
) -> Result<project_ai::AiProjectDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    project_ai::load_or_init(&workspace_path, &doc)
}

/// ADR 0028：项目工作区内实际存在的 AI 目录（只读检测），供「AI 目录」跳转区。
#[tauri::command]
pub fn ai_project_ai_dirs(
    app: AppHandle,
    workspace_path: String,
) -> Result<Vec<project_ai::ProjectAiDir>, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    project_ai::project_ai_dirs(&workspace_path, &doc)
}

/// ADR 0028：打开 Agent Target 的全局路径槽（Skills / Sessions / Rules / MCP）
/// 在系统资源管理器或默认程序中打开。后端完成 token 展开；不写入任何内容。
#[tauri::command]
pub async fn ai_agent_target_open_path(
    app: AppHandle,
    id: String,
    slot: String,
) -> Result<crate::open_with::OpenWithResult, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let target = doc
        .agent_targets
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| agents::ERR_AGENT_UNKNOWN.to_string())?;
    let expanded = agents::resolve_global_slot_path(target, &slot)?;
    if !std::path::Path::new(&expanded).exists() {
        return Err(agents::ERR_AGENT_PATH_NOT_FOUND.to_string());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        crate::open_with::open_existing_path(&expanded)
    })
    .await
    .map_err(|err| format!("ai_agent_target_open_path worker failed: {err}"))?;
    Ok(result)
}

#[tauri::command]
pub fn ai_project_ai_set_enabled_agents(
    app: AppHandle,
    workspace_path: String,
    enabled_agent_ids: Vec<String>,
) -> Result<project_ai::AiProjectDocument, String> {
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    project_ai::set_enabled_agents(&workspace_path, &doc, enabled_agent_ids)
}

/// 项目内已写入的 AFM 托管 MCP / Skill（只读扫描，供列表标记「已添加」）。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiProjectDeployments {
    pub mcp_ids: Vec<String>,
    pub skill_names: Vec<String>,
}

/// 扫描项目内已写入的 AFM 托管 MCP / Skill。
///
/// 「已添加」采用**交集**语义：条目必须存在于每一个「有目标路径的启用 Agent」中。
/// 这样新勾选一个 Agent 后，原本只在部分 Agent 的条目会重新显示为「未添加」，
/// 用户可再次勾选把它补齐到新 Agent；若用并集，只要任一 Agent 有文件就显示
/// 「已添加」，新 Agent 永远补不上文件。
fn scan_project_deployments(
    ws: &Path,
    doc: &AiConfigDocument,
    enabled: &[String],
) -> AiProjectDeployments {
    use std::collections::BTreeMap;
    use std::fs;

    let project_agents = doc
        .agent_targets
        .iter()
        // Agent Target 是源头：中央 enabled=false 的 Target 不算部署槽位（ADR 0010 修订）。
        .filter(|agent| agent.enabled && enabled.iter().any(|id| id == &agent.id));

    let mut mcp_hits: BTreeMap<String, usize> = BTreeMap::new();
    let mut skill_hits: BTreeMap<String, usize> = BTreeMap::new();
    let mut mcp_slots = 0usize;
    let mut skill_slots = 0usize;

    for agent in project_agents {
        let rel = agent.project_mcp_path.trim();
        if !rel.is_empty() {
            mcp_slots += 1;
            let file = crate::paths::join_relative(ws, rel);
            for id in mcp::managed_ids_in_file(&file) {
                *mcp_hits.entry(id).or_insert(0) += 1;
            }
        }
        let rel = agent.project_skills_dir.trim();
        if !rel.is_empty() {
            skill_slots += 1;
            let skills_dir = crate::paths::join_relative(ws, rel);
            if let Ok(entries) = fs::read_dir(&skills_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if deploy::is_afm_deployed_skill(&path) {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            *skill_hits.entry(name.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    let mcp_ids = mcp_hits
        .into_iter()
        .filter(|(_, hits)| *hits == mcp_slots)
        .map(|(id, _)| id)
        .collect();
    let skill_names = skill_hits
        .into_iter()
        .filter(|(_, hits)| *hits == skill_slots)
        .map(|(name, _)| name)
        .collect();
    AiProjectDeployments {
        mcp_ids,
        skill_names,
    }
}

#[tauri::command]
pub fn ai_project_deployments(
    app: AppHandle,
    workspace_path: String,
) -> Result<AiProjectDeployments, String> {
    let empty = AiProjectDeployments {
        mcp_ids: Vec::new(),
        skill_names: Vec::new(),
    };
    let ws = Path::new(workspace_path.trim());
    if !ws.is_dir() {
        return Ok(empty);
    }
    let dir = config_dir(&app)?;
    let doc = agents::load_from_path(&dir)?;
    let enabled = project_ai::enabled_agent_ids(&workspace_path, &doc)?;
    Ok(scan_project_deployments(ws, &doc, &enabled))
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
    let next =
        providers::upsert_compatible(&doc, id, display_name, base_url, api_key, default_model)?;
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
    fn shell_lists_all_sections_in_order() {
        let status = ai_addon_status();
        assert_eq!(status.layer, "ai-addon");
        assert_eq!(status.shell_version, AI_ADDON_SHELL_VERSION);
        assert_eq!(
            status
                .sections
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            AI_MENU_SECTION_IDS.to_vec()
        );
    }

    #[test]
    fn project_status_uses_intersection_across_enabled_agents() {
        use crate::ai::agents::AiConfigDocument;
        use std::fs;

        let ws = std::env::temp_dir().join(format!(
            "afm-ai-project-status-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&ws);
        fs::create_dir_all(&ws).unwrap();

        let mut doc = AiConfigDocument::empty();
        let a = crate::ai::agents::upsert_custom(
            &doc,
            Some("a".into()),
            "A".into(),
            String::new(),
            String::new(),
            String::new(),
            "a/mcp.json".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();
        doc = a;
        let mut b = doc.agent_targets[0].clone();
        b.id = "b".into();
        b.display_name = "B".into();
        b.project_mcp_path = "b/mcp.json".into();
        doc.agent_targets.push(b);
        // Agent Target 是源头：两个 agent 都需中央启用才计为部署槽位。
        doc = crate::ai::agents::set_enabled(&doc, "a", true).unwrap();
        doc = crate::ai::agents::set_enabled(&doc, "b", true).unwrap();

        // Only agent A has the managed MCP entry on disk.
        fs::create_dir_all(ws.join("a")).unwrap();
        fs::write(
            ws.join("a/mcp.json"),
            r#"{"mcpServers":{"afm__demo":{}}}"#,
        )
        .unwrap();

        // Enabled = [A]: demo should be "added".
        let only_a = scan_project_deployments(&ws, &doc, &["a".to_string()]);
        assert!(only_a.mcp_ids.iter().any(|id| id == "demo"));

        // Enabled = [A, B]: B has no file → demo must NOT be "added" (intersection).
        let both = scan_project_deployments(&ws, &doc, &["a".to_string(), "b".to_string()]);
        assert!(
            !both.mcp_ids.iter().any(|id| id == "demo"),
            "partial deploy must show as not-added so it can be re-checked"
        );

        let _ = fs::remove_dir_all(&ws);
    }

    /// Agent Target 是源头：中央禁用的 agent 不算部署槽位，即使项目启用集含它（ADR 0010 修订）。
    #[test]
    fn project_status_ignores_centrally_disabled_agents() {
        use crate::ai::agents::AiConfigDocument;
        use std::fs;

        let ws = std::env::temp_dir().join(format!(
            "afm-ai-project-status-disabled-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&ws);
        fs::create_dir_all(&ws).unwrap();

        let mut doc = AiConfigDocument::empty();
        doc = crate::ai::agents::upsert_custom(
            &doc,
            Some("on".into()),
            "On".into(),
            String::new(),
            String::new(),
            String::new(),
            "on/mcp.json".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();
        let mut off = doc.agent_targets[0].clone();
        off.id = "off".into();
        off.display_name = "Off".into();
        off.project_mcp_path = "off/mcp.json".into();
        doc.agent_targets.push(off);
        // 仅 on 中央启用；off 保持中央禁用。
        doc = crate::ai::agents::set_enabled(&doc, "on", true).unwrap();

        // 两个 agent 的项目文件都写入了托管键。
        fs::create_dir_all(ws.join("on")).unwrap();
        fs::write(
            ws.join("on/mcp.json"),
            r#"{"mcpServers":{"afm__demo":{}}}"#,
        )
        .unwrap();
        fs::create_dir_all(ws.join("off")).unwrap();
        fs::write(
            ws.join("off/mcp.json"),
            r#"{"mcpServers":{"afm__demo":{}}}"#,
        )
        .unwrap();

        // 项目启用集同时含 on 与 off：中央禁用的 off 不计为槽位，demo 应视为已添加。
        let scan = scan_project_deployments(&ws, &doc, &["on".to_string(), "off".to_string()]);
        assert!(scan.mcp_ids.iter().any(|id| id == "demo"));

        // 反例：项目启用集只剩中央禁用的 off → 任何条目都不算已添加。
        let only_off = scan_project_deployments(&ws, &doc, &["off".to_string()]);
        assert!(
            !only_off.mcp_ids.iter().any(|id| id == "demo"),
            "centrally disabled agent must not count as a deployment slot"
        );

        let _ = fs::remove_dir_all(&ws);
    }
}
