// SPDX-License-Identifier: GPL-3.0-or-later
//! Public MCP catalog + merge deploy (AIA-218 / A5 · ADR 0006).
//!
//! Catalog lives in APPDATA `ai-config.json` (not in Workspaces). Deploy merges
//! AFM-managed entries into target MCP JSON (`mcpServers`); never whole-file overwrite.
//! Managed key prefix: `afm__<id>`.

use super::agents::{
    expand_agent_path, AgentTarget, AiConfigDocument, McpCatalogEntry, AI_CONFIG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const AFM_MCP_KEY_PREFIX: &str = "afm__";

pub const ERR_MCP_ID_EMPTY: &str = "AI_MCP_ID_EMPTY";
pub const ERR_MCP_ID_INVALID: &str = "AI_MCP_ID_INVALID";
pub const ERR_MCP_UNKNOWN: &str = "AI_MCP_UNKNOWN";
pub const ERR_MCP_DEPLOY_NO_ENABLED: &str = "AI_MCP_DEPLOY_NO_ENABLED";
pub const ERR_MCP_TARGET_INVALID: &str = "AI_MCP_TARGET_INVALID";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpMergeReport {
    pub entry_id: String,
    pub written: Vec<String>,
    pub skipped: Vec<String>,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn validate_id(raw: &str) -> Result<String, String> {
    let id = raw.trim().to_ascii_lowercase();
    if id.is_empty() {
        return Err(ERR_MCP_ID_EMPTY.to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || id.starts_with('-')
        || id.ends_with('-')
    {
        return Err(ERR_MCP_ID_INVALID.to_string());
    }
    Ok(id)
}

pub fn managed_key(id: &str) -> String {
    format!("{AFM_MCP_KEY_PREFIX}{id}")
}

pub fn upsert_entry(
    document: &AiConfigDocument,
    id: Option<String>,
    display_name: String,
    server_json: String,
) -> Result<AiConfigDocument, String> {
    let name = display_name.trim();
    if name.is_empty() {
        return Err(ERR_MCP_ID_EMPTY.to_string());
    }
    // Validate JSON object
    let parsed: Value = serde_json::from_str(&server_json)
        .map_err(|err| format!("Invalid MCP server JSON: {err}"))?;
    if !parsed.is_object() {
        return Err("MCP server JSON must be an object".to_string());
    }
    let stamp = now_millis();
    let resolved = if let Some(raw) = id {
        validate_id(&raw)?
    } else {
        validate_id(
            &name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>(),
        )?
    };

    let mut next = document.clone();
    next.schema_version = AI_CONFIG_SCHEMA_VERSION;
    if let Some(existing) = next.mcp_catalog.iter_mut().find(|e| e.id == resolved) {
        existing.display_name = name.to_string();
        existing.server_json = parsed.to_string();
        existing.updated_at = stamp;
    } else {
        next.mcp_catalog.push(McpCatalogEntry {
            id: resolved,
            display_name: name.to_string(),
            server_json: parsed.to_string(),
            created_at: stamp,
            updated_at: stamp,
        });
    }
    next.mcp_catalog
        .sort_by(|a, b| a.display_name.to_ascii_lowercase().cmp(&b.display_name.to_ascii_lowercase()));
    Ok(next)
}

pub fn remove_entry(document: &AiConfigDocument, id: &str) -> Result<AiConfigDocument, String> {
    let id = validate_id(id)?;
    if !document.mcp_catalog.iter().any(|e| e.id == id) {
        return Err(ERR_MCP_UNKNOWN.to_string());
    }
    let mut next = document.clone();
    next.mcp_catalog.retain(|e| e.id != id);
    Ok(next)
}

fn load_mcp_file(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({ "mcpServers": {} }));
    }
    let text = fs::read_to_string(path).map_err(|err| format!("Failed to read MCP file: {err}"))?;
    if text.trim().is_empty() {
        return Ok(json!({ "mcpServers": {} }));
    }
    serde_json::from_str(&text).map_err(|err| format!("Failed to parse MCP JSON: {err}"))
}

fn mcp_servers_map(root: &mut Value) -> Result<&mut Map<String, Value>, String> {
    if !root.is_object() {
        return Err(ERR_MCP_TARGET_INVALID.to_string());
    }
    let obj = root.as_object_mut().unwrap();
    if !obj.contains_key("mcpServers") {
        obj.insert("mcpServers".into(), json!({}));
    }
    let servers = obj.get_mut("mcpServers").unwrap();
    if !servers.is_object() {
        return Err(ERR_MCP_TARGET_INVALID.to_string());
    }
    Ok(servers.as_object_mut().unwrap())
}

fn write_mcp_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("Failed to ensure MCP dir: {err}"))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("Failed to serialize MCP JSON: {err}"))?;
    fs::write(path, text).map_err(|err| format!("Failed to write MCP file: {err}"))?;
    Ok(())
}

fn merge_into_file(path: &Path, key: &str, server: &Value, remove: bool) -> Result<(), String> {
    let mut root = load_mcp_file(path)?;
    let servers = mcp_servers_map(&mut root)?;
    if remove {
        servers.remove(key);
    } else {
        servers.insert(key.to_string(), server.clone());
    }
    write_mcp_file(path, &root)
}

pub fn deploy_entry(
    document: &AiConfigDocument,
    entry_id: &str,
    project_workspace: Option<&str>,
) -> Result<McpMergeReport, String> {
    let id = validate_id(entry_id)?;
    let entry = document
        .mcp_catalog
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| ERR_MCP_UNKNOWN.to_string())?;
    let server: Value = serde_json::from_str(&entry.server_json)
        .map_err(|err| format!("Corrupt catalog server JSON: {err}"))?;
    let key = managed_key(&id);

    let enabled: Vec<&AgentTarget> = document.agent_targets.iter().filter(|a| a.enabled).collect();
    if enabled.is_empty() {
        return Err(ERR_MCP_DEPLOY_NO_ENABLED.to_string());
    }

    let mut written = Vec::new();
    let mut skipped = Vec::new();

    for agent in enabled {
        let global = expand_agent_path(&agent.global_mcp_path);
        if global.is_empty() {
            skipped.push(format!("{}: empty global MCP path", agent.id));
        } else {
            let path = PathBuf::from(&global);
            match merge_into_file(&path, &key, &server, false) {
                Ok(()) => written.push(normalize(&path)),
                Err(err) => skipped.push(format!("{}: {err}", agent.id)),
            }
        }

        if let Some(ws) = project_workspace {
            let rel = agent.project_mcp_path.trim();
            if rel.is_empty() {
                skipped.push(format!("{}: empty project MCP path", agent.id));
                continue;
            }
            let ws_path = PathBuf::from(ws.trim());
            if !ws_path.is_dir() {
                return Err(ERR_MCP_TARGET_INVALID.to_string());
            }
            let path = ws_path.join(rel.replace('/', "\\"));
            match merge_into_file(&path, &key, &server, false) {
                Ok(()) => written.push(normalize(&path)),
                Err(err) => skipped.push(format!("{}@project: {err}", agent.id)),
            }
        }
    }

    Ok(McpMergeReport {
        entry_id: id,
        written,
        skipped,
    })
}

pub fn undeploy_entry(
    document: &AiConfigDocument,
    entry_id: &str,
    project_workspace: Option<&str>,
) -> Result<McpMergeReport, String> {
    let id = validate_id(entry_id)?;
    if !document.mcp_catalog.iter().any(|e| e.id == id) {
        return Err(ERR_MCP_UNKNOWN.to_string());
    }
    let key = managed_key(&id);
    let enabled: Vec<&AgentTarget> = document.agent_targets.iter().filter(|a| a.enabled).collect();
    let mut written = Vec::new();
    let mut skipped = Vec::new();
    let dummy = json!({});

    for agent in enabled {
        let global = expand_agent_path(&agent.global_mcp_path);
        if !global.is_empty() {
            let path = PathBuf::from(&global);
            if path.exists() {
                match merge_into_file(&path, &key, &dummy, true) {
                    Ok(()) => written.push(normalize(&path)),
                    Err(err) => skipped.push(format!("{}: {err}", agent.id)),
                }
            }
        }
        if let Some(ws) = project_workspace {
            let rel = agent.project_mcp_path.trim();
            if rel.is_empty() {
                continue;
            }
            let path = PathBuf::from(ws).join(rel.replace('/', "\\"));
            if path.exists() {
                match merge_into_file(&path, &key, &dummy, true) {
                    Ok(()) => written.push(normalize(&path)),
                    Err(err) => skipped.push(format!("{}@project: {err}", agent.id)),
                }
            }
        }
    }

    Ok(McpMergeReport {
        entry_id: id,
        written,
        skipped,
    })
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agents::{load_from_path, save_to_path, set_enabled, upsert_custom};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-ai-mcp-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn merge_preserves_foreign_servers() {
        let dir = temp_dir("merge");
        let mcp_path = dir.join("mcp.json");
        fs::write(
            &mcp_path,
            r#"{ "mcpServers": { "user-tool": { "command": "keep" } } }"#,
        )
        .unwrap();

        let mut doc = load_from_path(&dir).unwrap();
        doc = upsert_custom(
            &doc,
            Some("mcp-agent".into()),
            "MCP Agent".into(),
            String::new(),
            String::new(),
            mcp_path.to_string_lossy().into(),
            r".cursor\mcp.json".into(),
            String::new(),
        )
        .unwrap();
        doc = set_enabled(&doc, "mcp-agent", true).unwrap();
        doc = upsert_entry(
            &doc,
            Some("demo".into()),
            "Demo MCP".into(),
            r#"{"command":"npx","args":["-y","demo"]}"#.into(),
        )
        .unwrap();
        save_to_path(&dir, &doc).unwrap();

        let report = deploy_entry(&doc, "demo", None).unwrap();
        assert!(!report.written.is_empty());

        let text = fs::read_to_string(&mcp_path).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        let servers = v.get("mcpServers").unwrap().as_object().unwrap();
        assert!(servers.contains_key("user-tool"));
        assert!(servers.contains_key("afm__demo"));
        assert_eq!(
            servers.get("user-tool").unwrap().get("command").unwrap(),
            "keep"
        );

        undeploy_entry(&doc, "demo", None).unwrap();
        let text = fs::read_to_string(&mcp_path).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        let servers = v.get("mcpServers").unwrap().as_object().unwrap();
        assert!(servers.contains_key("user-tool"));
        assert!(!servers.contains_key("afm__demo"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn catalog_round_trip() {
        let dir = temp_dir("catalog");
        let doc = load_from_path(&dir).unwrap();
        let next = upsert_entry(
            &doc,
            Some("fire".into()),
            "Fire".into(),
            r#"{"url":"http://127.0.0.1"}"#.into(),
        )
        .unwrap();
        assert_eq!(next.mcp_catalog.len(), 1);
        let gone = remove_entry(&next, "fire").unwrap();
        assert!(gone.mcp_catalog.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
