// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// vNext API version — intentionally incompatible with legacy marketplace packages.
pub const EXTENSION_HOST_API_VERSION: &str = "vnext.1";

/// Feature Pack 清单 API 版本。
pub const FEATURE_PACK_API_VERSION: &str = "feature-pack.1";

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
    /// 允许扩展拉起声明的 sidecar 子进程。
    SpawnSidecar,
}

impl ExtensionPermission {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "uiCommands" | "ui_commands" => Some(Self::UiCommands),
            "fsProvider" | "fs_provider" => Some(Self::FsProvider),
            "network" => Some(Self::Network),
            "spawnSidecar" | "spawn_sidecar" => Some(Self::SpawnSidecar),
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

/// Feature Pack 网关兼容范围（semver 字符串，本阶段仅存储）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GatewayCompat {
    pub min_version: String,
    pub max_version: String,
}

/// 侧栏导航贡献项。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SideNavContribution {
    pub id: String,
    pub label_key: String,
    pub to: String,
    pub order: i32,
}

/// 路由贡献项；`entry` 例如 `builtin:tool-runtime`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouteContribution {
    pub path: String,
    pub name: String,
    pub entry: String,
}

/// 命令贡献项。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandContribution {
    pub id: String,
    pub label_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

/// Sidecar 子进程贡献项。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SidecarContribution {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir_rel: Option<String>,
}

/// Feature Pack 贡献聚合。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeaturePackContributions {
    #[serde(default)]
    pub side_nav: Vec<SideNavContribution>,
    #[serde(default)]
    pub routes: Vec<RouteContribution>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub sidecars: Vec<SidecarContribution>,
}

/// Feature Pack 安装清单（`apiVersion` 为 `feature-pack.1`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeaturePackManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub contributions: FeaturePackContributions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway_compat: Option<GatewayCompat>,
}
