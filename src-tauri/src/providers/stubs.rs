// SPDX-License-Identifier: GPL-3.0-or-later

use super::provider::Provider;
use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ResolveResult, StatResult,
};

const LIST_CAP: &[ProviderCapability] = &[ProviderCapability::List];

/// 仅接口占位（ADR 0003）；内置列表不再注册，网关在线时由网关提供真实后端。
pub struct InterfaceOnlyProvider {
    id: &'static str,
    name: &'static str,
    description: &'static str,
}

impl InterfaceOnlyProvider {
    pub fn downloads() -> Self {
        Self {
            id: "downloads",
            name: "Downloads",
            description: "qBittorrent 下载任务树（需网关）",
        }
    }

    pub fn prowlarr() -> Self {
        Self {
            id: "prowlarr",
            name: "Prowlarr",
            description: "索引搜索（需网关）",
        }
    }
}

impl Provider for InterfaceOnlyProvider {
    fn backend_id(&self) -> &str {
        self.id
    }
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        self.description
    }
    fn capabilities(&self) -> &[ProviderCapability] {
        LIST_CAP
    }
    fn interface_only(&self) -> bool {
        true
    }

    fn health(&self) -> BackendInfo {
        BackendInfo {
            id: self.id.to_string(),
            name: self.name.to_string(),
            description: self.description.to_string(),
            healthy: false,
            capabilities: LIST_CAP
                .iter()
                .map(|capability| capability.as_str().to_string())
                .collect(),
            detail: Some("仅占位，需启动 awam-file 网关后使用".to_string()),
            interface_only: true,
            source: Some("builtin".to_string()),
        }
    }

    fn list(&self, uri: &AwamUri) -> DirListing {
        DirListing {
            uri: uri.to_string_uri(),
            backend: self.id.to_string(),
            entries: Vec::new(),
            error: Some(format!(
                "Provider '{}' is interface-only and not implemented yet",
                self.id
            )),
        }
    }

    fn stat(&self, _uri: &AwamUri) -> StatResult {
        StatResult {
            entry: None,
            error: Some(format!(
                "Provider '{}' is interface-only and not implemented yet",
                self.id
            )),
        }
    }

    fn resolve(&self, uri: &AwamUri) -> ResolveResult {
        ResolveResult {
            uri: uri.to_string_uri(),
            local_path: None,
            action: None,
            action_args: Default::default(),
            error: Some(format!(
                "Provider '{}' is interface-only and not implemented yet",
                self.id
            )),
        }
    }
}
