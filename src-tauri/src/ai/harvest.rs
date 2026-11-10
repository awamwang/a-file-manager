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
    path.to_string_lossy().replace('/', "\\")
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

/// 扫描 Agent 全局 MCP JSON，列出可导入候选（不写盘）。
pub fn scan_mcp_from_agent(
    document: &AiConfigDocument,
    agent_id: &str,
) -> Result<HarvestMcpScanReport, String> {
    let agent = require_agent(document, agent_id)?;
    let expanded = expand_agent_path(&agent.global_mcp_path);
    if expanded.is_empty() {
        return Err(ERR_HARVEST_PATH.to_string());
    }
    let path = PathBuf::from(&expanded);
    if !path.is_file() {
        return Err(ERR_HARVEST_PATH.to_string());
    }

    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read MCP file: {err}"))?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|err| format!("Invalid MCP JSON: {err}"))?;
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
    Ok((
        next,
        McpImportReport {
            added,
            skipped,
        },
    ))
}

/// 扫描 Agent 全局 Skills 目录（仅一层子目录含 SKILL.md）。
pub fn scan_skills_from_agent(
    config_dir: &Path,
    document: &AiConfigDocument,
    agent_id: &str,
) -> Result<HarvestSkillScanReport, String> {
    let agent = require_agent(document, agent_id)?;
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
