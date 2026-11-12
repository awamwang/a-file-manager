// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ResolveResult, StatResult,
};

/// Extension-layer FS provider (awam Adapter contract). Not Kernel disk browse.
pub trait Provider: Send + Sync {
    fn backend_id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    #[allow(dead_code)]
    fn capabilities(&self) -> &[ProviderCapability];
    /// 仅占位、尚无本机实现（ADR 0003）。
    #[allow(dead_code)]
    fn interface_only(&self) -> bool {
        false
    }

    fn health(&self) -> BackendInfo;

    fn list(&self, uri: &AwamUri) -> DirListing;

    fn stat(&self, uri: &AwamUri) -> StatResult {
        let listing = self.list(uri);
        if let Some(error) = listing.error {
            return StatResult {
                entry: None,
                error: Some(error),
            };
        }
        if uri.path.is_empty() {
            return StatResult {
                entry: Some(super::types::ProviderEntry {
                    name: self.backend_id().to_string(),
                    uri: uri.to_string_uri(),
                    is_dir: true,
                    is_file: false,
                    size: 0,
                    modified_time: 0,
                    local_path: None,
                    extra: Default::default(),
                    required_query: None,
                }),
                error: None,
            };
        }
        StatResult {
            entry: None,
            error: Some(format!("stat not specialized for {}", uri.to_string_uri())),
        }
    }

    fn resolve(&self, uri: &AwamUri) -> ResolveResult {
        ResolveResult {
            uri: uri.to_string_uri(),
            local_path: None,
            action: None,
            action_args: Default::default(),
            error: Some(format!("No local path for {}", uri.to_string_uri())),
        }
    }
}
