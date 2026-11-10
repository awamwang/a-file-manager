// SPDX-License-Identifier: GPL-3.0-or-later
//! Project Workspace AI 状态（ADR 0009 / 0010）。
//!
//! 存在 `.afm/ai-project.json`，不写入 Core `project.json`。
//! 惰性初始化：首次读取/部署时按中央 `defaultInProjects` 生成。

use super::agents::{AgentTarget, AiConfigDocument};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const AFM_DIR_NAME: &str = ".afm";
pub const AI_PROJECT_FILE_NAME: &str = "ai-project.json";
pub const AI_PROJECT_SCHEMA_VERSION: u32 = 1;

pub const ERR_PROJECT_AI_WORKSPACE: &str = "AI_PROJECT_WORKSPACE_INVALID";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiProjectDocument {
    pub schema_version: u32,
    pub enabled_agent_ids: Vec<String>,
    pub updated_at: u64,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn ai_project_path(workspace: &Path) -> PathBuf {
    workspace.join(AFM_DIR_NAME).join(AI_PROJECT_FILE_NAME)
}

fn require_workspace(workspace: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(workspace.trim());
    if !path.is_dir() {
        return Err(ERR_PROJECT_AI_WORKSPACE.to_string());
    }
    Ok(path)
}

fn default_enabled_ids(document: &AiConfigDocument) -> Vec<String> {
    document
        .agent_targets
        .iter()
        .filter(|t| t.default_in_projects)
        .map(|t| t.id.clone())
        .collect()
}

fn normalize_ids(ids: &[String], known: &[AgentTarget]) -> Vec<String> {
    let mut out: Vec<String> = ids
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .filter(|id| known.iter().any(|t| t.id == *id))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// 读取或惰性创建项目 AI 状态。
pub fn load_or_init(
    workspace: &str,
    central: &AiConfigDocument,
) -> Result<AiProjectDocument, String> {
    let root = require_workspace(workspace)?;
    let path = ai_project_path(&root);
    if path.is_file() {
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read ai-project.json: {err}"))?;
        let mut doc: AiProjectDocument = serde_json::from_str(&text)
            .map_err(|err| format!("Failed to parse ai-project.json: {err}"))?;
        doc.enabled_agent_ids = normalize_ids(&doc.enabled_agent_ids, &central.agent_targets);
        doc.schema_version = AI_PROJECT_SCHEMA_VERSION;
        return Ok(doc);
    }

    let doc = AiProjectDocument {
        schema_version: AI_PROJECT_SCHEMA_VERSION,
        enabled_agent_ids: default_enabled_ids(central),
        updated_at: now_millis(),
    };
    save(workspace, &doc)?;
    Ok(doc)
}

pub fn save(workspace: &str, document: &AiProjectDocument) -> Result<(), String> {
    let root = require_workspace(workspace)?;
    let dir = root.join(AFM_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|err| format!("Failed to create .afm: {err}"))?;
    let path = dir.join(AI_PROJECT_FILE_NAME);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize ai-project.json: {err}"))?;
    let tmp = dir.join(format!("{AI_PROJECT_FILE_NAME}.tmp"));
    fs::write(&tmp, &text).map_err(|err| format!("Failed to write ai-project.json: {err}"))?;
    fs::rename(&tmp, &path).map_err(|err| format!("Failed to replace ai-project.json: {err}"))?;
    Ok(())
}

pub fn set_enabled_agents(
    workspace: &str,
    central: &AiConfigDocument,
    enabled_agent_ids: Vec<String>,
) -> Result<AiProjectDocument, String> {
    let mut doc = load_or_init(workspace, central)?;
    doc.enabled_agent_ids = normalize_ids(&enabled_agent_ids, &central.agent_targets);
    doc.updated_at = now_millis();
    doc.schema_version = AI_PROJECT_SCHEMA_VERSION;
    save(workspace, &doc)?;
    Ok(doc)
}

/// 解析项目部署应使用的 Agent 列表。
pub fn resolve_project_agents<'a>(
    central: &'a AiConfigDocument,
    workspace: &str,
) -> Result<Vec<&'a AgentTarget>, String> {
    let project = load_or_init(workspace, central)?;
    let agents: Vec<&AgentTarget> = central
        .agent_targets
        .iter()
        .filter(|t| project.enabled_agent_ids.iter().any(|id| id == &t.id))
        .collect();
    Ok(agents)
}

/// 全局部署：仅 `enabled` 的 Target。
pub fn resolve_global_agents(central: &AiConfigDocument) -> Vec<&AgentTarget> {
    central
        .agent_targets
        .iter()
        .filter(|a| a.enabled)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agents::{load_from_path, set_enabled, update_target, AgentTargetPatch};

    fn temp_ws(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-ai-project-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lazy_init_from_default_in_projects() {
        let cfg = temp_ws("cfg");
        let ws = temp_ws("ws");
        let mut doc = load_from_path(&cfg).unwrap();
        doc = set_enabled(&doc, "cursor", true).unwrap();
        doc = update_target(
            &doc,
            "cursor",
            AgentTargetPatch {
                display_name: None,
                enabled: None,
                default_in_projects: Some(true),
                global_skills_dir: None,
                project_skills_dir: None,
                global_mcp_path: None,
                project_mcp_path: None,
                mcp_usage_prompt: None,
            },
        )
        .unwrap();
        let project = load_or_init(ws.to_str().unwrap(), &doc).unwrap();
        assert!(project.enabled_agent_ids.iter().any(|id| id == "cursor"));
        assert!(ai_project_path(&ws).is_file());
        let _ = fs::remove_dir_all(&cfg);
        let _ = fs::remove_dir_all(&ws);
    }
}
