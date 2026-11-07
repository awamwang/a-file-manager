// SPDX-License-Identifier: GPL-3.0-or-later
//! Agent Target registry (AIA-215 / A2 · ADR 0006).
//!
//! Unified targets for Skills dirs + MCP config paths (global + project-relative).
//! `mcp_usage_prompt` is a **placeholder** — stored, never injected (A2).
//! Discovery probes path existence; enable/custom are user actions. No Skill/MCP deploy (A3+/A5).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const AI_CONFIG_FILE_NAME: &str = "ai-config.json";
pub const AI_CONFIG_SCHEMA_VERSION: u32 = 1;

pub const ERR_AGENT_ID_EMPTY: &str = "AI_AGENT_ID_EMPTY";
pub const ERR_AGENT_ID_INVALID: &str = "AI_AGENT_ID_INVALID";
pub const ERR_AGENT_NAME_EMPTY: &str = "AI_AGENT_NAME_EMPTY";
pub const ERR_AGENT_UNKNOWN: &str = "AI_AGENT_UNKNOWN";
pub const ERR_AGENT_BUILTIN_REMOVE: &str = "AI_AGENT_BUILTIN_REMOVE";
pub const ERR_AGENT_ID_CONFLICT: &str = "AI_AGENT_ID_CONFLICT";

const MAX_ID_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentTarget {
    pub id: String,
    pub display_name: String,
    /// `"builtin"` | `"custom"`.
    pub kind: String,
    pub enabled: bool,
    /// True when global skills dir and/or global MCP path exists on disk.
    pub discovered: bool,
    pub global_skills_dir: String,
    pub project_skills_dir: String,
    pub global_mcp_path: String,
    pub project_mcp_path: String,
    /// Placeholder only — not injected into agents (ADR 0006 / A2).
    pub mcp_usage_prompt: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentTargetPatch {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub global_skills_dir: Option<String>,
    pub project_skills_dir: Option<String>,
    pub global_mcp_path: Option<String>,
    pub project_mcp_path: Option<String>,
    pub mcp_usage_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiConfigDocument {
    pub schema_version: u32,
    pub agent_targets: Vec<AgentTarget>,
    /// Empty → default `<config_dir>\.agents\skills` (ADR 0005 / A3).
    #[serde(default)]
    pub skills_library_root: String,
    /// Public MCP catalog (ADR 0006 / A5) — APPDATA only, not per-Workspace.
    #[serde(default)]
    pub mcp_catalog: Vec<McpCatalogEntry>,
    /// AI model Providers MVP (A7) — presets + Compatible; not FS Providers.
    #[serde(default)]
    pub ai_providers: Vec<crate::ai::providers::AiProvider>,
}

/// One public MCP server entry (stored in APPDATA AI config).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpCatalogEntry {
    pub id: String,
    pub display_name: String,
    /// Opaque JSON object text for the server config (command/args/url/env…).
    pub server_json: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl AiConfigDocument {
    pub fn empty() -> Self {
        Self {
            schema_version: AI_CONFIG_SCHEMA_VERSION,
            agent_targets: Vec::new(),
            skills_library_root: String::new(),
            mcp_catalog: Vec::new(),
            ai_providers: Vec::new(),
        }
    }
}

struct BuiltinDef {
    id: &'static str,
    display_name: &'static str,
    global_skills_dir: &'static str,
    project_skills_dir: &'static str,
    global_mcp_path: &'static str,
    project_mcp_path: &'static str,
}

/// Built-in Agent Target presets (path conventions from ADR 0006 + local agent layouts).
/// Paths use `%USERPROFILE%` / `%APPDATA%` tokens expanded at discovery time.
const BUILTIN_DEFS: &[BuiltinDef] = &[
    BuiltinDef {
        id: "cursor",
        display_name: "Cursor",
        global_skills_dir: r"%USERPROFILE%\.cursor\skills",
        project_skills_dir: r".cursor\skills",
        global_mcp_path: r"%USERPROFILE%\.cursor\mcp.json",
        project_mcp_path: r".cursor\mcp.json",
    },
    BuiltinDef {
        id: "claude-code",
        display_name: "Claude Code",
        global_skills_dir: r"%USERPROFILE%\.claude\skills",
        project_skills_dir: r".claude\skills",
        global_mcp_path: r"%USERPROFILE%\.claude.json",
        project_mcp_path: r".mcp.json",
    },
    BuiltinDef {
        id: "cline",
        display_name: "Cline / agents",
        global_skills_dir: r"%USERPROFILE%\.agents\skills",
        project_skills_dir: r".agents\skills",
        global_mcp_path: "",
        project_mcp_path: "",
    },
    BuiltinDef {
        id: "opencode",
        display_name: "OpenCode",
        global_skills_dir: r"%USERPROFILE%\.config\opencode\skills",
        project_skills_dir: r".opencode\skills",
        global_mcp_path: r"%USERPROFILE%\.config\opencode\opencode.json",
        project_mcp_path: r".opencode\opencode.json",
    },
    BuiltinDef {
        id: "trae",
        display_name: "Trae",
        global_skills_dir: r"%USERPROFILE%\.trae\skills",
        project_skills_dir: r".trae\skills",
        global_mcp_path: r"%USERPROFILE%\.trae\mcp.json",
        project_mcp_path: r".trae\mcp.json",
    },
    BuiltinDef {
        id: "qoder",
        display_name: "Qoder",
        global_skills_dir: r"%USERPROFILE%\.qoder\skills",
        project_skills_dir: r".qoder\skills",
        global_mcp_path: r"%USERPROFILE%\.qoder\mcp.json",
        project_mcp_path: r".qoder\mcp.json",
    },
    BuiltinDef {
        id: "workbuddy",
        display_name: "WorkBuddy",
        global_skills_dir: r"%USERPROFILE%\.workbuddy\skills",
        project_skills_dir: r".workbuddy\skills",
        global_mcp_path: r"%USERPROFILE%\.workbuddy\mcp.json",
        project_mcp_path: r".workbuddy\mcp.json",
    },
    BuiltinDef {
        id: "vscode",
        display_name: "VS Code / Continue",
        global_skills_dir: r"%USERPROFILE%\.continue\skills",
        project_skills_dir: r".continue\skills",
        global_mcp_path: r"%USERPROFILE%\.continue\config.json",
        project_mcp_path: r".continue\config.json",
    },
];

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn ai_config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(AI_CONFIG_FILE_NAME)
}

/// Expand `%USERPROFILE%` / `%APPDATA%` / `%LOCALAPPDATA%` (Windows-oriented).
pub fn expand_agent_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = trimmed.to_string();
    for (key, fallback_env) in [
        ("%USERPROFILE%", "USERPROFILE"),
        ("%APPDATA%", "APPDATA"),
        ("%LOCALAPPDATA%", "LOCALAPPDATA"),
    ] {
        if out.to_ascii_uppercase().contains(&key.to_ascii_uppercase()) {
            if let Ok(val) = std::env::var(fallback_env) {
                // Case-insensitive replace of the token.
                let lower = out.to_ascii_lowercase();
                let token = key.to_ascii_lowercase();
                if let Some(idx) = lower.find(&token) {
                    out = format!("{}{}{}", &out[..idx], val, &out[idx + key.len()..]);
                }
            }
        }
    }
    if let Some(rest) = out.strip_prefix('~') {
        if let Ok(home) = std::env::var("USERPROFILE") {
            let rest = rest.trim_start_matches(['/', '\\']);
            out = if rest.is_empty() {
                home
            } else {
                format!("{home}\\{rest}")
            };
        }
    }
    out.replace('/', "\\")
}

pub fn path_exists_expanded(raw: &str) -> bool {
    let expanded = expand_agent_path(raw);
    if expanded.is_empty() {
        return false;
    }
    Path::new(&expanded).exists()
}

pub fn refresh_discovered(target: &mut AgentTarget) {
    target.discovered = path_exists_expanded(&target.global_skills_dir)
        || path_exists_expanded(&target.global_mcp_path);
}

fn validate_id(raw: &str) -> Result<String, String> {
    let id = raw.trim().to_ascii_lowercase();
    if id.is_empty() {
        return Err(ERR_AGENT_ID_EMPTY.to_string());
    }
    if id.len() > MAX_ID_LEN {
        return Err(ERR_AGENT_ID_INVALID.to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ERR_AGENT_ID_INVALID.to_string());
    }
    if id.starts_with('-') || id.ends_with('-') {
        return Err(ERR_AGENT_ID_INVALID.to_string());
    }
    Ok(id)
}

fn builtin_target(def: &BuiltinDef, stamp: u64) -> AgentTarget {
    let mut target = AgentTarget {
        id: def.id.to_string(),
        display_name: def.display_name.to_string(),
        kind: "builtin".to_string(),
        enabled: false,
        discovered: false,
        global_skills_dir: def.global_skills_dir.to_string(),
        project_skills_dir: def.project_skills_dir.to_string(),
        global_mcp_path: def.global_mcp_path.to_string(),
        project_mcp_path: def.project_mcp_path.to_string(),
        mcp_usage_prompt: String::new(),
        created_at: stamp,
        updated_at: stamp,
    };
    refresh_discovered(&mut target);
    target
}

/// Ensure all builtins exist in the document; refresh discovery flags.
pub fn sync_builtins_and_discover(document: &AiConfigDocument) -> AiConfigDocument {
    let stamp = now_millis();
    let mut next = document.clone();
    next.schema_version = AI_CONFIG_SCHEMA_VERSION;

    for def in BUILTIN_DEFS {
        if let Some(existing) = next.agent_targets.iter_mut().find(|t| t.id == def.id) {
            // Keep user overrides for paths / prompt / enabled; refresh discovery only.
            if existing.kind != "custom" {
                existing.kind = "builtin".to_string();
            }
            refresh_discovered(existing);
        } else {
            next.agent_targets.push(builtin_target(def, stamp));
        }
    }

    for target in &mut next.agent_targets {
        refresh_discovered(target);
    }

    next.agent_targets.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    next
}

pub fn load_from_path(config_dir: &Path) -> Result<AiConfigDocument, String> {
    let path = ai_config_path(config_dir);
    let document = if !path.exists() {
        AiConfigDocument::empty()
    } else {
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read AI config: {err}"))?;
        serde_json::from_str(&text)
            .map_err(|err| format!("Failed to parse AI config JSON: {err}"))?
    };
    let synced = crate::ai::mcp::sync_seed_catalog(&crate::ai::providers::sync_presets(
        &sync_builtins_and_discover(&document),
    ));
    if synced != document {
        save_to_path(config_dir, &synced)?;
    }
    Ok(synced)
}

pub fn save_to_path(config_dir: &Path, document: &AiConfigDocument) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create AI config directory: {err}"))?;
    let path = ai_config_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize AI config: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write AI config: {err}"))?;
    Ok(())
}

pub fn set_enabled(
    document: &AiConfigDocument,
    id: &str,
    enabled: bool,
) -> Result<AiConfigDocument, String> {
    let mut next = document.clone();
    let target = next
        .agent_targets
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| ERR_AGENT_UNKNOWN.to_string())?;
    target.enabled = enabled;
    target.updated_at = now_millis();
    refresh_discovered(target);
    Ok(next)
}

pub fn update_target(
    document: &AiConfigDocument,
    id: &str,
    patch: AgentTargetPatch,
) -> Result<AiConfigDocument, String> {
    let mut next = document.clone();
    let target = next
        .agent_targets
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| ERR_AGENT_UNKNOWN.to_string())?;

    if let Some(name) = patch.display_name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ERR_AGENT_NAME_EMPTY.to_string());
        }
        target.display_name = trimmed.to_string();
    }
    if let Some(enabled) = patch.enabled {
        target.enabled = enabled;
    }
    if let Some(v) = patch.global_skills_dir {
        target.global_skills_dir = v.trim().to_string();
    }
    if let Some(v) = patch.project_skills_dir {
        target.project_skills_dir = v.trim().replace('/', "\\");
    }
    if let Some(v) = patch.global_mcp_path {
        target.global_mcp_path = v.trim().to_string();
    }
    if let Some(v) = patch.project_mcp_path {
        target.project_mcp_path = v.trim().replace('/', "\\");
    }
    if let Some(v) = patch.mcp_usage_prompt {
        // Placeholder field — store as-is; never inject (A2).
        target.mcp_usage_prompt = v;
    }
    target.updated_at = now_millis();
    refresh_discovered(target);
    Ok(next)
}

pub fn upsert_custom(
    document: &AiConfigDocument,
    id: Option<String>,
    display_name: String,
    global_skills_dir: String,
    project_skills_dir: String,
    global_mcp_path: String,
    project_mcp_path: String,
    mcp_usage_prompt: String,
) -> Result<AiConfigDocument, String> {
    let name = display_name.trim();
    if name.is_empty() {
        return Err(ERR_AGENT_NAME_EMPTY.to_string());
    }
    let stamp = now_millis();
    let resolved_id = if let Some(raw) = id {
        validate_id(&raw)?
    } else {
        let slug = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let collapsed = slug
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        validate_id(&format!("custom-{collapsed}"))?
    };

    let mut next = document.clone();
    if let Some(existing) = next.agent_targets.iter_mut().find(|t| t.id == resolved_id) {
        if existing.kind == "builtin" {
            return Err(ERR_AGENT_ID_CONFLICT.to_string());
        }
        existing.display_name = name.to_string();
        existing.global_skills_dir = global_skills_dir.trim().to_string();
        existing.project_skills_dir = project_skills_dir.trim().replace('/', "\\");
        existing.global_mcp_path = global_mcp_path.trim().to_string();
        existing.project_mcp_path = project_mcp_path.trim().replace('/', "\\");
        existing.mcp_usage_prompt = mcp_usage_prompt;
        existing.updated_at = stamp;
        refresh_discovered(existing);
        return Ok(next);
    }

    if BUILTIN_DEFS.iter().any(|d| d.id == resolved_id) {
        return Err(ERR_AGENT_ID_CONFLICT.to_string());
    }

    let mut target = AgentTarget {
        id: resolved_id,
        display_name: name.to_string(),
        kind: "custom".to_string(),
        enabled: false,
        discovered: false,
        global_skills_dir: global_skills_dir.trim().to_string(),
        project_skills_dir: project_skills_dir.trim().replace('/', "\\"),
        global_mcp_path: global_mcp_path.trim().to_string(),
        project_mcp_path: project_mcp_path.trim().replace('/', "\\"),
        mcp_usage_prompt,
        created_at: stamp,
        updated_at: stamp,
    };
    refresh_discovered(&mut target);
    next.agent_targets.push(target);
    next.agent_targets.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
    });
    Ok(next)
}

pub fn remove_custom(document: &AiConfigDocument, id: &str) -> Result<AiConfigDocument, String> {
    let Some(target) = document.agent_targets.iter().find(|t| t.id == id) else {
        return Err(ERR_AGENT_UNKNOWN.to_string());
    };
    if target.kind != "custom" {
        return Err(ERR_AGENT_BUILTIN_REMOVE.to_string());
    }
    let mut next = document.clone();
    next.agent_targets.retain(|t| t.id != id);
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-ai-agents-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn seeds_builtins_and_persists() {
        let dir = temp_dir("seed");
        let doc = load_from_path(&dir).unwrap();
        assert!(doc.agent_targets.len() >= BUILTIN_DEFS.len());
        assert!(doc.agent_targets.iter().any(|t| t.id == "cursor"));
        assert!(ai_config_path(&dir).is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn enable_and_update_prompt_placeholder() {
        let dir = temp_dir("enable");
        let doc = load_from_path(&dir).unwrap();
        let enabled = set_enabled(&doc, "cursor", true).unwrap();
        let cursor = enabled.agent_targets.iter().find(|t| t.id == "cursor").unwrap();
        assert!(cursor.enabled);

        let patched = update_target(
            &enabled,
            "cursor",
            AgentTargetPatch {
                display_name: None,
                enabled: None,
                global_skills_dir: None,
                project_skills_dir: None,
                global_mcp_path: None,
                project_mcp_path: None,
                mcp_usage_prompt: Some("PLACEHOLDER — do not inject".into()),
            },
        )
        .unwrap();
        let cursor = patched.agent_targets.iter().find(|t| t.id == "cursor").unwrap();
        assert_eq!(cursor.mcp_usage_prompt, "PLACEHOLDER — do not inject");
        // Field is stored only; no injection helper exists in this module.
        assert!(!cursor.mcp_usage_prompt.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn custom_upsert_and_remove() {
        let dir = temp_dir("custom");
        let doc = load_from_path(&dir).unwrap();
        let with = upsert_custom(
            &doc,
            Some("my-agent".into()),
            "My Agent".into(),
            r"%USERPROFILE%\.my-agent\skills".into(),
            r".my-agent\skills".into(),
            r"%USERPROFILE%\.my-agent\mcp.json".into(),
            r".my-agent\mcp.json".into(),
            String::new(),
        )
        .unwrap();
        assert!(with.agent_targets.iter().any(|t| t.id == "my-agent" && t.kind == "custom"));

        let removed = remove_custom(&with, "my-agent").unwrap();
        assert!(!removed.agent_targets.iter().any(|t| t.id == "my-agent"));

        assert_eq!(
            remove_custom(&removed, "cursor").unwrap_err(),
            ERR_AGENT_BUILTIN_REMOVE
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_custom_id_colliding_with_builtin() {
        let dir = temp_dir("conflict");
        let doc = load_from_path(&dir).unwrap();
        let err = upsert_custom(
            &doc,
            Some("cursor".into()),
            "Fake Cursor".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap_err();
        assert_eq!(err, ERR_AGENT_ID_CONFLICT);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_userprofile_token() {
        let expanded = expand_agent_path(r"%USERPROFILE%\.cursor\skills");
        assert!(!expanded.contains("%USERPROFILE%"));
        assert!(expanded.to_ascii_lowercase().ends_with(r"\.cursor\skills"));
    }
}
