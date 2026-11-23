// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

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

/// 扩展宿主（Feature Pack）状态概览。vNext 扩展 / marketplace / fs_hooks 通道已移除，
/// 计数一律由本机 Feature Pack 派生（installed=全部 pack，enabled=启用，crashed=清单损坏）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionHostStatus {
    pub api_version: String,
    pub installed_count: usize,
    pub enabled_count: usize,
    pub crashed_count: usize,
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

/// 共享运行时种类（AFM 统一提供、同类产物共用；ADR 0022）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SharedRuntime {
    Python,
    Node,
}

/// Sidecar 运行时来源（ADR 0022：shared-first + per-artifact 自带优先 precedence）。
/// **自带则用自带，否则用共享**——`self` 是默认档，旧 `pack.json` 缺 `runtime` 字段即按此解析。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SidecarRuntime {
    /// 产物自带解释器（如 PyInstaller onefile）：清单里的 `command` 即 argv0，`args` 原样。
    #[default]
    #[serde(rename = "self")]
    Self_,
    /// 用 AFM 提供的共享 Python / Node 运行时：把 `command` 当脚本/入口前置到 `args` 之前。
    Shared(SharedRuntime),
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
    /// 运行时档（默认 `self`，向后兼容无此字段的旧清单）。
    #[serde(default)]
    pub runtime: SidecarRuntime,
    /// 静态环境变量（ADR 0022 / M4-A）。spawn 时注入子进程环境。
    /// 值可含 `${afm:settings:<key>}` 占位，由托管命令层从 AFM 设置解析（密钥不落 pack.json）。
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub env: std::collections::BTreeMap<String, String>,
    /// 子进程读取监听端口的环境变量名（ADR 0022 / 端口回退）。设了它，AFM 在托管 spawn 前
    /// 可在目标端口被**非本服务**占用时顺延到一个可绑端口，并把最终端口写进该 env；
    /// 缺省则不启用端口回退（子进程用其自身默认端口，行为同迁移前）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_env: Option<String>,
}

/// Pack 声明的配置项值类型（服务配置页据此选择控件；映射到 settings schema 的 value_kind）。
/// `Secret` 与 `String` 都落 settings 文档为字符串，仅 UI 用密码框 + 日志脱敏区分。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigValueKind {
    #[default]
    String,
    Secret,
    Number,
    Boolean,
    Enum,
}

/// Pack 需要的配置项贡献（发现机制核心）：provider 在 `pack.json` 声明「需要用户填什么」，
/// AFM 服务配置页据此自动渲染对应字段，用户填写后写进 settings，并在托管 spawn 时按 `env`
/// 注入子进程、或作为 AFM 客户端凭据使用。值本身仍存 settings 文档（动态并入 schema overlay，
/// 标 `hidden` 不进主设置页），故 sanitize 保留、迁移既有 `power.*` key 时不丢用户已存值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigContribution {
    /// pack 内稳定 id，如 `prowlarr-api-key`。缺省 settingKey 时用于生成 `ext.<packId>.<id>`。
    pub id: String,
    /// 展示标题（provider 自带文案，与 `pack.name` 内联风格一致；不强制走 AFM i18n）。
    #[serde(default)]
    pub title: String,
    /// 标题的 AFM i18n key（优先于 `title`）。迁移既有字段时指向 `settings.fields.<key>.title`
    /// 以保持多语言；provider 自带文案的字段留空、回退 `title`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_key: Option<String>,
    /// 展示说明。
    #[serde(default)]
    pub description: String,
    /// 说明的 AFM i18n key（优先于 `description`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_key: Option<String>,
    /// 值类型（默认 `string`）。
    #[serde(default)]
    pub kind: ConfigValueKind,
    /// `kind=enum` 时的候选值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// 是否必填（前端据此提示，空值标红；不阻断保存）。
    #[serde(default)]
    pub required: bool,
    /// 托管（managed）模式下该字段只读/由宿主派生（如服务连接地址随 managed 端口回退变化）。
    /// 仅影响服务配置页渲染：managed 启用对应 pack 时输入框只读，config 模式仍可编辑。
    #[serde(default)]
    pub managed_readonly: bool,
    /// 字符串化默认值（缺省 settingKey 时 overlay 据此注册 settings schema 默认）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// 注入到托管 sidecar 进程的环境变量名。设了它，AFM 在 managed spawn 前把
    /// `settings[settingKey]` 写入子进程环境的该变量（不覆盖 `sidecars.env` 显式占位）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    /// 绑定到的 settings key。缺省时宿主生成 `ext.<packId>.<id>` 命名空间 key。
    /// 迁移既有字段时显式绑定到 `power.*`，复用内置 schema、兼容用户已存值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_key: Option<String>,
    /// 同 pack 内字段排序。
    #[serde(default)]
    pub order: i32,
    /// 聚合时由宿主写入（清单内声明无效），供服务配置页按 pack 分组渲染。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<String>,
    /// 聚合时由宿主写入（清单内声明无效），分组卡片标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_name: Option<String>,
}

/// 扩展页动态分区贡献项（ADR-0023）：Pack 启用后注入 ExtensionsView 左侧导航。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionsSectionContribution {
    pub id: String,
    pub label_key: String,
    /// builtinEntries 中的组件加载键（如 "builtin:tool-runtime"）。
    pub entry: String,
    #[serde(default)]
    pub order: i32,
}

/// 搜索面板静态条目贡献项；聚合时宿主写入 `packId` / `packName`（清单内声明无效）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntryContribution {
    pub id: String,
    pub label_key: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_name: Option<String>,
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
    #[serde(default)]
    pub config: Vec<ConfigContribution>,
    #[serde(default)]
    pub search: Vec<SearchEntryContribution>,
    #[serde(default)]
    pub extensions_sections: Vec<ExtensionsSectionContribution>,
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

/// 已安装 Feature Pack 记录。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: ExtensionRuntimeState,
    pub permissions: Vec<ExtensionPermission>,
    pub install_path: String,
    #[serde(default)]
    pub last_error: Option<String>,
}

/// 启用态 Feature Pack 的聚合贡献。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedContributions {
    #[serde(default)]
    pub side_nav: Vec<SideNavContribution>,
    #[serde(default)]
    pub routes: Vec<RouteContribution>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub sidecars: Vec<SidecarContribution>,
    #[serde(default)]
    pub config: Vec<ConfigContribution>,
    #[serde(default)]
    pub search: Vec<SearchEntryContribution>,
    #[serde(default)]
    pub extensions_sections: Vec<ExtensionsSectionContribution>,
}

/// Feature Pack 安装结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackInstallResult {
    pub success: bool,
    pub pack: Option<InstalledPackRecord>,
    pub error: Option<String>,
}

/// feed「可安装 Store」列表项（ADR 0022）：供扩展页列出 feed manifest 中的 pack 并一键安装。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallablePack {
    pub pack_id: String,
    pub name: String,
    pub version: String,
    /// pack 安装包（zip）直链；受 feed allow-list 约束。
    pub install_url: String,
    pub sha256: String,
    /// 本机是否已装同名 pack。
    pub installed: bool,
    /// 已装时的本机版本（未装 `None`），前端据此显示「可更新」。
    #[serde(default)]
    pub installed_version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_pack_manifest_json_round_trip_camel_case() {
        let json = r#"{
            "id": "demo.pack",
            "name": "Demo",
            "version": "1.0.0",
            "apiVersion": "feature-pack.1",
            "description": "round-trip",
            "permissions": ["spawnSidecar", "uiCommands"],
            "contributions": {
                "sideNav": [
                    {
                        "id": "nav-tools",
                        "labelKey": "nav.tools",
                        "to": "/tools",
                        "order": 10
                    }
                ],
                "routes": [
                    {
                        "path": "/tools",
                        "name": "tools",
                        "entry": "builtin:tool-runtime"
                    }
                ],
                "commands": [
                    {
                        "id": "open-tools",
                        "labelKey": "cmd.openTools",
                        "route": "/tools"
                    }
                ],
                "sidecars": [
                    {
                        "id": "tool-sidecar",
                        "command": "node",
                        "args": ["server.js"],
                        "workingDirRel": "sidecar"
                    }
                ],
                "search": [
                    {
                        "id": "open-demo",
                        "labelKey": "demo.openEntry",
                        "keywords": ["demo", "entry"],
                        "route": "/tools"
                    }
                ]
            },
            "gatewayCompat": {
                "minVersion": "0.9.0",
                "maxVersion": "1.0.0"
            }
        }"#;

        let parsed: FeaturePackManifest =
            serde_json::from_str(json).expect("deserialize FeaturePackManifest");
        let re_json = serde_json::to_string(&parsed).expect("serialize FeaturePackManifest");
        let round_trip: FeaturePackManifest =
            serde_json::from_str(&re_json).expect("re-deserialize FeaturePackManifest");

        assert_eq!(parsed, round_trip);
        assert_eq!(parsed.api_version, FEATURE_PACK_API_VERSION);
        assert_eq!(parsed.contributions.side_nav[0].label_key, "nav.tools");
        assert_eq!(
            parsed.contributions.sidecars[0].working_dir_rel.as_deref(),
            Some("sidecar")
        );
        assert_eq!(parsed.contributions.search[0].label_key, "demo.openEntry");
        assert_eq!(parsed.contributions.search[0].keywords.len(), 2);
        assert_eq!(parsed.contributions.search[0].pack_id, None);
        assert_eq!(
            parsed
                .gateway_compat
                .as_ref()
                .map(|g| g.min_version.as_str()),
            Some("0.9.0")
        );
        assert!(re_json.contains("\"sideNav\""));
        assert!(re_json.contains("\"labelKey\""));
        assert!(re_json.contains("\"workingDirRel\""));
        assert!(re_json.contains("\"gatewayCompat\""));
    }
}
