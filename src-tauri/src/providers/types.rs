// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_required_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueryParam {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "default_required_true")]
    pub required: bool,
}

/// 对 AFM 隐藏的后端 id（Everything `search`）；与 `local` 一样大小写不敏感。
pub fn is_hidden_provider_backend(id: &str) -> bool {
    id.eq_ignore_ascii_case("search")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderCapability {
    List,
    Stat,
    Read,
    Search,
    Resolve,
}

impl ProviderCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Stat => "stat",
            Self::Read => "read",
            Self::Search => "search",
            Self::Resolve => "resolve",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub healthy: bool,
    pub capabilities: Vec<String>,
    pub detail: Option<String>,
    /// 仅占位、尚无本机实现（ADR 0003 interface-only）。
    #[serde(default)]
    pub interface_only: bool,
    /// `"gateway"` 来自 awam-file HTTP；`"builtin"` 为本进程内置。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderEntry {
    pub name: String,
    pub uri: String,
    pub is_dir: bool,
    pub is_file: bool,
    #[serde(default)]
    pub size: u64,
    /// Unix milliseconds (align with Navigator thin list).
    #[serde(default)]
    pub modified_time: u64,
    #[serde(default)]
    pub local_path: Option<String>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_query: Option<Vec<QueryParam>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DirListing {
    pub uri: String,
    pub backend: String,
    pub entries: Vec<ProviderEntry>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_query: Option<Vec<QueryParam>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatResult {
    pub entry: Option<ProviderEntry>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveResult {
    pub uri: String,
    #[serde(default)]
    pub local_path: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub action_args: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwamUri {
    pub backend: String,
    pub path: String,
    pub query: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_param_required_defaults_true_and_camel_case() {
        let parsed: QueryParam = serde_json::from_value(serde_json::json!({
            "key": "q",
            "label": "Query"
        }))
        .unwrap();
        assert_eq!(parsed.key, "q");
        assert_eq!(parsed.label.as_deref(), Some("Query"));
        assert!(parsed.required);

        let serialized = serde_json::to_value(&parsed).unwrap();
        assert_eq!(serialized["key"], "q");
        assert_eq!(serialized["label"], "Query");
        assert_eq!(serialized["required"], true);
        assert!(serialized.get("required_query").is_none());
    }

    #[test]
    fn dir_listing_serializes_required_query_camel_case() {
        let listing = DirListing {
            uri: "awam://hf/remote".to_string(),
            backend: "hf".to_string(),
            entries: Vec::new(),
            error: None,
            required_query: Some(vec![QueryParam {
                key: "q".to_string(),
                label: Some("Query".to_string()),
                required: true,
            }]),
        };
        let value = serde_json::to_value(&listing).unwrap();
        assert_eq!(value["requiredQuery"][0]["key"], "q");
        assert_eq!(value["requiredQuery"][0]["label"], "Query");
        assert_eq!(value["requiredQuery"][0]["required"], true);
    }
}
