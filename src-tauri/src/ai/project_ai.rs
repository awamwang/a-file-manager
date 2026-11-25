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

/// 只读：项目启用的 Agent id（文件不存在时回落到中央默认集），不创建 ai-project.json。
pub fn enabled_agent_ids(
    workspace: &str,
    central: &AiConfigDocument,
) -> Result<Vec<String>, String> {
    let root = require_workspace(workspace)?;
    let path = ai_project_path(&root);
    if !path.is_file() {
        return Ok(default_enabled_ids(central));
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read ai-project.json: {err}"))?;
    let doc: AiProjectDocument = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse ai-project.json: {err}"))?;
    Ok(normalize_ids(&doc.enabled_agent_ids, &central.agent_targets))
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
///
/// Agent Target 是源头：中央 `enabled=false` 的 Target 一律不参与项目 fan-out，
/// 即使 `.afm/ai-project.json` 仍存有该 id（存储保留，重新启用后自动恢复）。
pub fn resolve_project_agents<'a>(
    central: &'a AiConfigDocument,
    workspace: &str,
) -> Result<Vec<&'a AgentTarget>, String> {
    let project = load_or_init(workspace, central)?;
    let agents: Vec<&AgentTarget> = central
        .agent_targets
        .iter()
        .filter(|t| t.enabled && project.enabled_agent_ids.iter().any(|id| id == &t.id))
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

/// 项目工作区内实际存在的 AI 目录条目（只读检测，ADR 0028）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAiDir {
    /// 相对路径（如 `.afm`、`.cursor/skills`）。
    pub name: String,
    /// 工作区内的绝对路径。
    pub path: String,
}

/// 汇总 `.afm` + 该项目启用 Agent 的 `projectSkillsDir` / `projectRulesDir`
/// 真实配置（项目启用集 ∩ 中央 enabled，ADR 0010 修订），只返回磁盘上存在的目录。
/// 只读：不创建任何文件/目录（用 `enabled_agent_ids` 而非 `load_or_init`）。
pub fn project_ai_dirs(
    workspace: &str,
    central: &AiConfigDocument,
) -> Result<Vec<ProjectAiDir>, String> {
    let root = require_workspace(workspace)?;
    let enabled = enabled_agent_ids(workspace, central)?;

    let mut rel_dirs: Vec<String> = vec![AFM_DIR_NAME.to_string()];
    for agent in central
        .agent_targets
        .iter()
        .filter(|t| t.enabled && enabled.iter().any(|id| id == &t.id))
    {
        for rel in [&agent.project_skills_dir, &agent.project_rules_dir] {
            let trimmed = rel.trim().trim_end_matches(['/', '\\']);
            if !trimmed.is_empty() {
                rel_dirs.push(trimmed.to_string());
            }
        }
    }
    rel_dirs.sort();
    rel_dirs.dedup();

    let mut out = Vec::new();
    for rel in rel_dirs {
        let abs = root.join(&rel);
        if abs.is_dir() {
            out.push(ProjectAiDir {
                name: rel,
                path: abs.to_string_lossy().into_owned(),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agents::{
        load_from_path, set_enabled, update_target, upsert_custom, AgentTargetPatch,
    };

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
                global_sessions_dir: None,
                global_rules_path: None,
                project_rules_dir: None,
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

    /// Agent Target 是源头：中央禁用的 Target 不参与项目 fan-out，重新启用后自动恢复（ADR 0010 修订）。
    #[test]
    fn resolve_project_agents_gated_by_central_enabled() {
        let cfg = temp_ws("cfg-gate");
        let ws = temp_ws("ws-gate");
        let mut doc = load_from_path(&cfg).unwrap();
        doc = upsert_custom(
            &doc,
            Some("gate-a".into()),
            "Gate A".into(),
            String::new(),
            "skills".into(),
            String::new(),
            "mcp.json".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();
        doc = set_enabled(&doc, "gate-a", true).unwrap();
        set_enabled_agents(ws.to_str().unwrap(), &doc, vec!["gate-a".to_string()]).unwrap();

        let agents = resolve_project_agents(&doc, ws.to_str().unwrap()).unwrap();
        assert!(agents.iter().any(|t| t.id == "gate-a"));

        // 中央关闭：项目启用集文件仍存有 gate-a（暂存），但 fan-out 必须排除它。
        let disabled = set_enabled(&doc, "gate-a", false).unwrap();
        assert!(fs::read_to_string(ai_project_path(&ws))
            .unwrap()
            .contains("gate-a"));
        let agents = resolve_project_agents(&disabled, ws.to_str().unwrap()).unwrap();
        assert!(
            agents.iter().all(|t| t.id != "gate-a"),
            "centrally disabled agent must be excluded from project fan-out"
        );

        // 重新启用：无需重勾，自动恢复参与 fan-out。
        let re_enabled = set_enabled(&disabled, "gate-a", true).unwrap();
        let agents = resolve_project_agents(&re_enabled, ws.to_str().unwrap()).unwrap();
        assert!(agents.iter().any(|t| t.id == "gate-a"));

        let _ = fs::remove_dir_all(&cfg);
        let _ = fs::remove_dir_all(&ws);
    }

    /// ADR 0028：项目 AI 目录只读检测——`.afm` + 启用 Agent 的
    /// projectSkillsDir/projectRulesDir 中磁盘上真实存在的目录。
    #[test]
    fn project_ai_dirs_lists_existing_dirs_readonly() {
        let cfg = temp_ws("cfg-dirs");
        let ws = temp_ws("ws-dirs");
        let mut doc = load_from_path(&cfg).unwrap();
        doc = upsert_custom(
            &doc,
            Some("dirs-a".into()),
            "Dirs A".into(),
            String::new(),
            "skills".into(),
            String::new(),
            "mcp.json".into(),
            String::new(),
            String::new(),
            "rules".into(),
            String::new(),
        )
        .unwrap();
        doc = set_enabled(&doc, "dirs-a", true).unwrap();
        set_enabled_agents(ws.to_str().unwrap(), &doc, vec!["dirs-a".to_string()]).unwrap();

        // 未创建任何目录：只报 .afm（enabled_agent_ids 读路径会因 load_or_init 缺席而回落——
        // set_enabled_agents 已写 .afm/ai-project.json，因此 .afm 存在）。
        fs::create_dir_all(ws.join("skills")).unwrap();
        fs::create_dir_all(ws.join("rules")).unwrap();
        let dirs = project_ai_dirs(ws.to_str().unwrap(), &doc).unwrap();
        let names: Vec<&str> = dirs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&".afm"), "afm dir expected: {names:?}");
        assert!(names.contains(&"skills"), "agent skills dir expected: {names:?}");
        assert!(names.contains(&"rules"), "agent rules dir expected: {names:?}");
        // 不存在的目录（mcp.json 是文件路径，不计）不出现。
        assert!(!names.iter().any(|n| n.contains("mcp")));
        for d in &dirs {
            assert!(Path::new(&d.path).is_dir(), "reported dir must exist: {}", d.path);
            assert!(d.path.starts_with(ws.to_str().unwrap()), "path must be inside ws");
        }

        // 磁盘上无 skills 目录时不列出。
        fs::remove_dir_all(ws.join("skills")).unwrap();
        let dirs = project_ai_dirs(ws.to_str().unwrap(), &doc).unwrap();
        assert!(!dirs.iter().any(|d| d.name == "skills"));

        let _ = fs::remove_dir_all(&cfg);
        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn project_ai_dirs_rejects_invalid_workspace() {
        let cfg = temp_ws("cfg-dirs-invalid");
        let doc = load_from_path(&cfg).unwrap();
        let err = project_ai_dirs("Z:\\definitely\\not\\a\\dir", &doc).unwrap_err();
        assert_eq!(err, ERR_PROJECT_AI_WORKSPACE);
        let _ = fs::remove_dir_all(&cfg);
    }
}
