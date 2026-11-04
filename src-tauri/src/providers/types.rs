// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// True when Day-1 ships only a reserved slot (ADR 0003 interface-only).
    #[serde(default)]
    pub interface_only: bool,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DirListing {
    pub uri: String,
    pub backend: String,
    pub entries: Vec<ProviderEntry>,
    #[serde(default)]
    pub error: Option<String>,
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
