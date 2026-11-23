// SPDX-License-Identifier: GPL-3.0-or-later
//! 从已配置 Agent Target 的**全局**路径手动 Harvest 到中央仓库。

use super::agents::{expand_agent_path, AiConfigDocument};
use super::mcp::{self, McpImportReport};
use super::skills::{self, LibrarySkill, SkillsLibraryStatus, SKILL_MANIFEST_FILE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const ERR_HARVEST_AGENT_UNKNOWN: &str = "AI_HARVEST_AGENT_UNKNOWN";
pub const ERR_HARVEST_AGENT_NOT_ENABLED: &str = "AI_HARVEST_AGENT_NOT_ENABLED";
pub const ERR_HARVEST_PATH: &str = "AI_HARVEST_PATH_INVALID";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarvestMcpCandidate {
    pub id: String,
    pub display_name: String,
    pub server_json: String,
    pub already_in_catalog: bool,
    pub source_agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarvestSkillCandidate {
    pub name: String,
    pub path: String,
    pub already_in_library: bool,
    pub source_agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarvestMcpScanReport {
    pub agent_id: String,
    pub source_path: String,
    pub candidates: Vec<HarvestMcpCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarvestSkillScanReport {
    pub agent_id: String,
    pub source_path: String,
    pub candidates: Vec<HarvestSkillCandidate>,
}

fn normalize(path: &Path) -> String {
    // Reports carry a real path; never Windows-ify POSIX separators (7.20).
    crate::paths::to_native_fs_path(&path.to_string_lossy())
}

fn require_agent<'a>(
    document: &'a AiConfigDocument,
    agent_id: &str,
) -> Result<&'a super::agents::AgentTarget, String> {
    document
        .agent_targets
        .iter()
        .find(|t| t.id == agent_id)
        .ok_or_else(|| ERR_HARVEST_AGENT_UNKNOWN.to_string())
}

/// Agent Target 是源头：Harvest 源必须是中央 `enabled` 的 Agent（ADR 0010 修订）。
fn require_enabled_agent<'a>(
    document: &'a AiConfigDocument,
    agent_id: &str,
) -> Result<&'a super::agents::AgentTarget, String> {
    let agent = require_agent(document, agent_id)?;
    if !agent.enabled {
        return Err(ERR_HARVEST_AGENT_NOT_ENABLED.to_string());
    }
    Ok(agent)
}

/// 扫描 Agent 全局 MCP JSON，列出可导入候选（不写盘）。
pub fn scan_mcp_from_agent(
    document: &AiConfigDocument,
    agent_id: &str,
) -> Result<HarvestMcpScanReport, String> {
    let agent = require_enabled_agent(document, agent_id)?;
    let expanded = expand_agent_path(&agent.global_mcp_path);
    if expanded.is_empty() {
        return Err(ERR_HARVEST_PATH.to_string());
    }
    let path = PathBuf::from(&expanded);
    if !path.is_file() {
        return Err(ERR_HARVEST_PATH.to_string());
    }

    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read MCP file: {err}"))?;
    let root: Value =
        serde_json::from_str(&text).map_err(|err| format!("Invalid MCP JSON: {err}"))?;
    let servers = root
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .ok_or_else(|| mcp::ERR_MCP_IMPORT_INVALID.to_string())?;

    let mut candidates = Vec::new();
    for (raw_id, server) in servers {
        if !server.is_object() {
            continue;
        }
        // 跳过本产品已托管的键，避免回环导入。
        if raw_id.starts_with(mcp::AFM_MCP_KEY_PREFIX) {
            continue;
        }
        let id = raw_id.trim().to_ascii_lowercase();
        if id.is_empty() {
            continue;
        }
        let server_json = serde_json::to_string(server)
            .map_err(|err| format!("Failed to serialize server JSON: {err}"))?;
        let already = document.mcp_catalog.iter().any(|e| e.id == id);
        candidates.push(HarvestMcpCandidate {
            id: id.clone(),
            display_name: raw_id.trim().to_string(),
            server_json,
            already_in_catalog: already,
            source_agent_id: agent.id.clone(),
        });
    }
    candidates.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(HarvestMcpScanReport {
        agent_id: agent.id.clone(),
        source_path: normalize(&path),
        candidates,
    })
}

/// 将勾选的 MCP 候选写入中央目录（已存在则跳过）。
pub fn import_mcp_candidates(
    document: &AiConfigDocument,
    candidates: &[HarvestMcpCandidate],
) -> Result<(AiConfigDocument, McpImportReport), String> {
    // Agent Target 是源头：候选的来源 Agent 必须仍是中央 enabled 的。
    for c in candidates {
        require_enabled_agent(document, &c.source_agent_id)?;
    }
    let mut next = document.clone();
    let mut added = Vec::new();
    let mut skipped = Vec::new();
    for c in candidates {
        if next.mcp_catalog.iter().any(|e| e.id == c.id) {
            skipped.push(c.id.clone());
            continue;
        }
        match mcp::upsert_entry(
            &next,
            Some(c.id.clone()),
            c.display_name.clone(),
            c.server_json.clone(),
        ) {
            Ok(doc) => {
                next = doc;
                added.push(c.id.clone());
            }
            Err(_) => skipped.push(c.id.clone()),
        }
    }
    Ok((next, McpImportReport { added, skipped }))
}

/// 扫描 Agent 全局 Skills 目录（仅一层子目录含 SKILL.md）。
pub fn scan_skills_from_agent(
    config_dir: &Path,
    document: &AiConfigDocument,
    agent_id: &str,
) -> Result<HarvestSkillScanReport, String> {
    let agent = require_enabled_agent(document, agent_id)?;
    let expanded = expand_agent_path(&agent.global_skills_dir);
    if expanded.is_empty() {
        return Err(ERR_HARVEST_PATH.to_string());
    }
    let root = PathBuf::from(&expanded);
    if !root.is_dir() {
        return Err(ERR_HARVEST_PATH.to_string());
    }

    let library_root = skills::resolve_skills_library_root(config_dir, document);
    let existing = skills::list_skills(&library_root).unwrap_or_default();

    let mut candidates = Vec::new();
    let entries = fs::read_dir(&root).map_err(|err| format!("Failed to read skills dir: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed to read skills entry: {err}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join(SKILL_MANIFEST_FILE).is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let already = existing.iter().any(|s| s.name == name);
        candidates.push(HarvestSkillCandidate {
            name,
            path: normalize(&path),
            already_in_library: already,
            source_agent_id: agent.id.clone(),
        });
    }
    candidates.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(HarvestSkillScanReport {
        agent_id: agent.id.clone(),
        source_path: normalize(&root),
        candidates,
    })
}

/// 将勾选的 Skill 路径导入中央库（已存在则跳过）。
pub fn import_skill_candidates(
    config_dir: &Path,
    document: &AiConfigDocument,
    candidates: &[HarvestSkillCandidate],
) -> Result<(SkillsLibraryStatus, Vec<LibrarySkill>, Vec<String>), String> {
    // Agent Target 是源头：候选的来源 Agent 必须仍是中央 enabled 的。
    for c in candidates {
        require_enabled_agent(document, &c.source_agent_id)?;
    }
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut last_status = skills::library_status(config_dir, document)?;

    for c in candidates {
        if last_status.skills.iter().any(|s| s.name == c.name) {
            skipped.push(c.name.clone());
            continue;
        }
        match skills::import_skill(config_dir, document, &c.path, Some(c.name.clone())) {
            Ok((status, skill)) => {
                last_status = status;
                imported.push(skill);
            }
            Err(_) => skipped.push(c.name.clone()),
        }
    }

    Ok((last_status, imported, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agents::{set_enabled, upsert_custom, AiConfigDocument};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-ai-harvest-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Agent Target 是源头：中央禁用的 Agent 不能作为 Harvest 源，启用后恢复（ADR 0010 修订）。
    #[test]
    fn harvest_scan_rejects_centrally_disabled_agent() {
        let dir = temp_dir("gate");
        let mcp_file = dir.join("mcp.json");
        fs::write(&mcp_file, r#"{"mcpServers":{"demo":{}}}"#).unwrap();
        let mut doc = AiConfigDocument::empty();
        doc = upsert_custom(
            &doc,
            Some("gated".into()),
            "Gated".into(),
            String::new(),
            String::new(),
            mcp_file.to_string_lossy().into(),
            String::new(),
            String::new(),
        )
        .unwrap();
        // 中央未启用：扫描必须被拒绝。
        let err = scan_mcp_from_agent(&doc, "gated").unwrap_err();
        assert!(err.contains(ERR_HARVEST_AGENT_NOT_ENABLED));
        // 中央启用后：同一配置可正常扫描。
        doc = set_enabled(&doc, "gated", true).unwrap();
        let report = scan_mcp_from_agent(&doc, "gated").unwrap();
        assert!(report.candidates.iter().any(|c| c.id == "demo"));
        let _ = fs::remove_dir_all(&dir);
    }

    /// 导入侧同样受源头闸门约束：候选的来源 Agent 被中央禁用时拒绝写入中央仓库。
    #[test]
    fn harvest_import_rejects_candidates_from_disabled_agent() {
        let dir = temp_dir("imp");
        let mut doc = AiConfigDocument::empty();
        doc = upsert_custom(
            &doc,
            Some("off-agent".into()),
            "Off Agent".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();

        let mcp_candidate = HarvestMcpCandidate {
            id: "demo".into(),
            display_name: "demo".into(),
            server_json: "{}".into(),
            already_in_catalog: false,
            source_agent_id: "off-agent".into(),
        };
        let err = import_mcp_candidates(&doc, &[mcp_candidate]).unwrap_err();
        assert!(err.contains(ERR_HARVEST_AGENT_NOT_ENABLED));

        let skill_candidate = HarvestSkillCandidate {
            name: "demo".into(),
            path: dir.to_string_lossy().into(),
            already_in_library: false,
            source_agent_id: "off-agent".into(),
        };
        let err = import_skill_candidates(&dir, &doc, &[skill_candidate]).unwrap_err();
        assert!(err.contains(ERR_HARVEST_AGENT_NOT_ENABLED));

        let _ = fs::remove_dir_all(&dir);
    }
}
