// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// vNext API version — intentionally incompatible with legacy marketplace packages.
pub const EXTENSION_HOST_API_VERSION: &str = "vnext.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionRuntimeState {
    Installed,
    Enabled,
    Disabled,
    Crashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionPermission {
    UiCommands,
    FsProvider,
    Network,
}

impl ExtensionPermission {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "uiCommands" | "ui_commands" => Some(Self::UiCommands),
            "fsProvider" | "fs_provider" => Some(Self::FsProvider),
            "network" => Some(Self::Network),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionManifestVNext {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Declared FS provider backend ids (metadata; ProviderRegistry is FS authority).
    #[serde(default)]
    pub fs_providers: Vec<String>,
    /// Declared UI / JS command ids.
    #[serde(default)]
    pub ui_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledExtensionRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: ExtensionRuntimeState,
    pub permissions: Vec<ExtensionPermission>,
    pub fs_providers: Vec<String>,
    pub ui_commands: Vec<String>,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceCatalog {
    pub api_version: String,
    /// Always empty in S8 — marketplace deliberately vacant; no GitHub fetch.
    pub extensions: Vec<Value>,
    pub empty: bool,
    pub detail: String,
    pub fetches_remote: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub success: bool,
    pub extension: Option<InstalledExtensionRecord>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiCommandRequest {
    pub extension_id: String,
    pub command_id: String,
    #[serde(default)]
    pub args: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiCommandResult {
    pub ok: bool,
    pub extension_id: String,
    pub command_id: String,
    #[serde(default)]
    pub payload: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
    /// True when the extension was marked crashed but the host stayed alive.
    #[serde(default)]
    pub isolated_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FsProviderContribution {
    pub extension_id: String,
    pub backend_id: String,
    /// Contribution is registered with the host; ProviderRegistry remains the FS authority.
    pub registered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionHostStatus {
    pub api_version: String,
    pub installed_count: usize,
    pub enabled_count: usize,
    pub crashed_count: usize,
    pub marketplace_empty: bool,
    pub fetches_remote: bool,
    pub fs_provider_hooks: usize,
}
