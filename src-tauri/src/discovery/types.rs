// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryEngine {
    Everything,
    Fallback,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryStatus {
    pub everything_available: bool,
    pub fallback_enabled: bool,
    pub active_engine: DiscoveryEngine,
    pub degraded: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryHit {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySearchResult {
    pub query: String,
    pub engine: DiscoveryEngine,
    pub total_count: u64,
    pub offset: u64,
    pub hits: Vec<DiscoveryHit>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySearchOptions {
    #[serde(default)]
    pub offset: u64,
    #[serde(default)]
    pub limit: Option<u64>,
}

impl Default for DiscoverySearchOptions {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: None,
        }
    }
}

pub const DEFAULT_SEARCH_LIMIT: u64 = 100;
pub const MAX_SEARCH_LIMIT: u64 = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EverythingProbe {
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EverythingQueryResult {
    pub total_count: u64,
    pub offset: u64,
    pub hits: Vec<DiscoveryHit>,
}
