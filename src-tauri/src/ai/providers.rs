// SPDX-License-Identifier: GPL-3.0-or-later
//! AI Providers MVP (AIA-220 / A7).
//!
//! Mainstream presets + Compatible (Base URL / API Key). No billing / usage panel.
//! Stored in APPDATA `ai-config.json` — separate from file-system Providers.

use crate::ai::agents::AiConfigDocument;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ERR_PROVIDER_ID_EMPTY: &str = "AI_PROVIDER_ID_EMPTY";
pub const ERR_PROVIDER_ID_INVALID: &str = "AI_PROVIDER_ID_INVALID";
pub const ERR_PROVIDER_NAME_EMPTY: &str = "AI_PROVIDER_NAME_EMPTY";
pub const ERR_PROVIDER_UNKNOWN: &str = "AI_PROVIDER_UNKNOWN";
pub const ERR_PROVIDER_BUILTIN_REMOVE: &str = "AI_PROVIDER_BUILTIN_REMOVE";
pub const ERR_PROVIDER_ID_CONFLICT: &str = "AI_PROVIDER_ID_CONFLICT";

const MAX_ID_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiProvider {
    pub id: String,
    pub display_name: String,
    /// `"preset"` | `"compatible"`.
    pub kind: String,
    pub enabled: bool,
    pub base_url: String,
    /// Stored locally in APPDATA; never logged. Empty allowed for presets until filled.
    pub api_key: String,
    /// Optional default model id hint (display only in MVP).
    pub default_model: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderPatch {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub default_model: Option<String>,
}

struct PresetDef {
    id: &'static str,
    display_name: &'static str,
    base_url: &'static str,
    default_model: &'static str,
}

const PRESETS: &[PresetDef] = &[
    PresetDef {
        id: "openai",
        display_name: "OpenAI",
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-4o",
    },
    PresetDef {
        id: "anthropic",
        display_name: "Anthropic",
        base_url: "https://api.anthropic.com",
        default_model: "claude-sonnet-4-5",
    },
    PresetDef {
        id: "azure-openai",
        display_name: "Azure OpenAI",
        base_url: "https://YOUR_RESOURCE.openai.azure.com",
        default_model: "gpt-4o",
    },
    PresetDef {
        id: "deepseek",
        display_name: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        default_model: "deepseek-chat",
    },
    PresetDef {
        id: "moonshot",
        display_name: "Moonshot (Kimi)",
        base_url: "https://api.moonshot.cn/v1",
        default_model: "moonshot-v1-auto",
    },
    PresetDef {
        id: "groq",
        display_name: "Groq",
        base_url: "https://api.groq.com/openai/v1",
        default_model: "llama-3.3-70b-versatile",
    },
];

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn normalize_id(raw: &str) -> Result<String, String> {
    let id = raw.trim().to_ascii_lowercase();
    if id.is_empty() {
        return Err(ERR_PROVIDER_ID_EMPTY.to_string());
    }
    if id.len() > MAX_ID_LEN
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ERR_PROVIDER_ID_INVALID.to_string());
    }
    Ok(id)
}

fn is_preset_id(id: &str) -> bool {
    PRESETS.iter().any(|p| p.id == id)
}

fn preset_row(def: &PresetDef, existing: Option<&AiProvider>) -> AiProvider {
    let ts = now_ms();
    if let Some(prev) = existing {
        AiProvider {
            id: def.id.to_string(),
            display_name: def.display_name.to_string(),
            kind: "preset".into(),
            enabled: prev.enabled,
            // Keep user overrides for URL/key/model; seed blanks from preset.
            base_url: if prev.base_url.trim().is_empty() {
                def.base_url.to_string()
            } else {
                prev.base_url.clone()
            },
            api_key: prev.api_key.clone(),
            default_model: if prev.default_model.trim().is_empty() {
                def.default_model.to_string()
            } else {
                prev.default_model.clone()
            },
            created_at: prev.created_at,
            updated_at: prev.updated_at,
        }
    } else {
        AiProvider {
            id: def.id.to_string(),
            display_name: def.display_name.to_string(),
            kind: "preset".into(),
            enabled: false,
            base_url: def.base_url.to_string(),
            api_key: String::new(),
            default_model: def.default_model.to_string(),
            created_at: ts,
            updated_at: ts,
        }
    }
}

/// Ensure all presets exist; preserve custom Compatible rows and user edits.
pub fn sync_presets(doc: &AiConfigDocument) -> AiConfigDocument {
    let mut next = doc.clone();
    let mut synced = Vec::new();
    for def in PRESETS {
        let prev = next.ai_providers.iter().find(|p| p.id == def.id);
        synced.push(preset_row(def, prev));
    }
    for row in &next.ai_providers {
        if row.kind == "compatible" && !is_preset_id(&row.id) {
            synced.push(row.clone());
        }
    }
    next.ai_providers = synced;
    next
}

pub fn set_enabled(
    doc: &AiConfigDocument,
    id: &str,
    enabled: bool,
) -> Result<AiConfigDocument, String> {
    let mut next = sync_presets(doc);
    let row = next
        .ai_providers
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| ERR_PROVIDER_UNKNOWN.to_string())?;
    row.enabled = enabled;
    row.updated_at = now_ms();
    Ok(next)
}

pub fn update_provider(
    doc: &AiConfigDocument,
    id: &str,
    patch: AiProviderPatch,
) -> Result<AiConfigDocument, String> {
    let mut next = sync_presets(doc);
    let row = next
        .ai_providers
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| ERR_PROVIDER_UNKNOWN.to_string())?;
    if let Some(name) = patch.display_name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(ERR_PROVIDER_NAME_EMPTY.to_string());
        }
        if row.kind == "compatible" {
            row.display_name = name;
        }
    }
    if let Some(enabled) = patch.enabled {
        row.enabled = enabled;
    }
    if let Some(url) = patch.base_url {
        row.base_url = url.trim().to_string();
    }
    if let Some(key) = patch.api_key {
        row.api_key = key.trim().to_string();
    }
    if let Some(model) = patch.default_model {
        row.default_model = model.trim().to_string();
    }
    row.updated_at = now_ms();
    Ok(next)
}

/// Upsert a Compatible provider (custom Base URL / API Key).
pub fn upsert_compatible(
    doc: &AiConfigDocument,
    id: Option<String>,
    display_name: String,
    base_url: String,
    api_key: String,
    default_model: String,
) -> Result<AiConfigDocument, String> {
    let name = display_name.trim().to_string();
    if name.is_empty() {
        return Err(ERR_PROVIDER_NAME_EMPTY.to_string());
    }
    let id = match id {
        Some(raw) => normalize_id(&raw)?,
        None => {
            let slug: String = name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect();
            normalize_id(&slug)?
        }
    };
    if is_preset_id(&id) {
        return Err(ERR_PROVIDER_ID_CONFLICT.to_string());
    }

    let mut next = sync_presets(doc);
    let ts = now_ms();
    if let Some(row) = next.ai_providers.iter_mut().find(|p| p.id == id) {
        if row.kind != "compatible" {
            return Err(ERR_PROVIDER_ID_CONFLICT.to_string());
        }
        row.display_name = name;
        row.base_url = base_url.trim().to_string();
        row.api_key = api_key.trim().to_string();
        row.default_model = default_model.trim().to_string();
        row.updated_at = ts;
    } else {
        next.ai_providers.push(AiProvider {
            id,
            display_name: name,
            kind: "compatible".into(),
            enabled: true,
            base_url: base_url.trim().to_string(),
            api_key: api_key.trim().to_string(),
            default_model: default_model.trim().to_string(),
            created_at: ts,
            updated_at: ts,
        });
    }
    Ok(next)
}

pub fn remove_compatible(doc: &AiConfigDocument, id: &str) -> Result<AiConfigDocument, String> {
    if is_preset_id(id) {
        return Err(ERR_PROVIDER_BUILTIN_REMOVE.to_string());
    }
    let mut next = sync_presets(doc);
    let before = next.ai_providers.len();
    next.ai_providers.retain(|p| p.id != id);
    if next.ai_providers.len() == before {
        return Err(ERR_PROVIDER_UNKNOWN.to_string());
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_seeds_presets() {
        let doc = sync_presets(&AiConfigDocument::empty());
        assert!(doc.ai_providers.iter().any(|p| p.id == "openai"));
        assert!(doc.ai_providers.iter().any(|p| p.id == "anthropic"));
        assert!(doc.ai_providers.iter().all(|p| p.kind == "preset"));
        assert!(doc.ai_providers.iter().all(|p| !p.enabled));
    }

    #[test]
    fn compatible_round_trip_preserves_presets() {
        let doc = sync_presets(&AiConfigDocument::empty());
        let with = upsert_compatible(
            &doc,
            Some("my-proxy".into()),
            "My Proxy".into(),
            "https://example.com/v1".into(),
            "sk-test".into(),
            "gpt-4o".into(),
        )
        .unwrap();
        assert_eq!(
            with.ai_providers
                .iter()
                .filter(|p| p.kind == "compatible")
                .count(),
            1
        );
        let enabled = set_enabled(&with, "openai", true).unwrap();
        assert!(enabled
            .ai_providers
            .iter()
            .find(|p| p.id == "openai")
            .unwrap()
            .enabled);
        assert!(enabled
            .ai_providers
            .iter()
            .any(|p| p.id == "my-proxy" && p.api_key == "sk-test"));

        let removed = remove_compatible(&enabled, "my-proxy").unwrap();
        assert!(!removed.ai_providers.iter().any(|p| p.id == "my-proxy"));
        assert_eq!(
            remove_compatible(&removed, "openai").unwrap_err(),
            ERR_PROVIDER_BUILTIN_REMOVE
        );
    }
}
