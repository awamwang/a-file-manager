// SPDX-License-Identifier: GPL-3.0-or-later
//! `awam://browser` 薄客户端：书签解析在 awam-file（Chrome/Edge Reader 注册表）。
//!
//! 网关在线时由 `providers::gateway` 代理 list/stat/resolve（含 `open_url`）。
//! 本模块仅作内置回退：网关离线时给出明确不可用提示，不再读取本机 Bookmarks JSON。
//! 应用内文件夹钉选（`favorites.json`）与本后端无关。

use super::provider::Provider;
use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ProviderEntry, ResolveResult, StatResult,
};
use std::collections::HashMap;

const CAPABILITIES: &[ProviderCapability] = &[
    ProviderCapability::List,
    ProviderCapability::Stat,
    ProviderCapability::Resolve,
];

const UNAVAILABLE: &str = "浏览器书签由 awam-file 网关提供。请启动网关（默认 http://127.0.0.1:18787）后重试。";

pub struct BrowserProvider;

impl BrowserProvider {
    pub fn system_default() -> Self {
        Self
    }

    fn unavailable_listing(uri: &AwamUri) -> DirListing {
        DirListing {
            uri: uri.to_string_uri(),
            backend: "browser".to_string(),
            entries: Vec::new(),
            error: Some(UNAVAILABLE.to_string()),
            required_query: None,
        }
    }
}

impl Provider for BrowserProvider {
    fn backend_id(&self) -> &str {
        "browser"
    }
    fn name(&self) -> &str {
        "Browser"
    }
    fn description(&self) -> &str {
        "Browser bookmarks via awam-file (Chrome/Edge first; no history)"
    }
    fn capabilities(&self) -> &[ProviderCapability] {
        CAPABILITIES
    }

    fn health(&self) -> BackendInfo {
        BackendInfo {
            id: "browser".to_string(),
            name: "Browser".to_string(),
            description: self.description().to_string(),
            healthy: false,
            capabilities: CAPABILITIES
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
            detail: Some(UNAVAILABLE.to_string()),
            interface_only: false,
            source: Some("builtin".to_string()),
        }
    }

    fn list(&self, uri: &AwamUri) -> DirListing {
        Self::unavailable_listing(uri)
    }

    fn stat(&self, uri: &AwamUri) -> StatResult {
        StatResult {
            entry: Some(ProviderEntry {
                name: self.backend_id().to_string(),
                uri: uri.to_string_uri(),
                is_dir: true,
                is_file: false,
                size: 0,
                modified_time: 0,
                local_path: None,
                extra: HashMap::new(),
                required_query: None,
            }),
            error: Some(UNAVAILABLE.to_string()),
        }
    }

    fn resolve(&self, uri: &AwamUri) -> ResolveResult {
        ResolveResult {
            uri: uri.to_string_uri(),
            local_path: None,
            action: None,
            action_args: HashMap::new(),
            error: Some(UNAVAILABLE.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_stub_does_not_parse_bookmarks() {
        let provider = BrowserProvider::system_default();
        assert!(!provider.health().healthy);
        let listing = provider.list(&AwamUri {
            backend: "browser".into(),
            path: String::new(),
            query: HashMap::new(),
        });
        assert!(listing.entries.is_empty());
        assert!(listing
            .error
            .as_deref()
            .is_some_and(|e| e.contains("awam-file")));
        let resolved = provider.resolve(&AwamUri {
            backend: "browser".into(),
            path: "edge/Default/bookmark_bar/10".into(),
            query: HashMap::new(),
        });
        assert!(resolved.action.is_none());
        assert!(resolved
            .error
            .as_deref()
            .is_some_and(|e| e.contains("awam-file")));
    }
}
