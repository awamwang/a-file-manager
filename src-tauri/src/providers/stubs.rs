// SPDX-License-Identifier: GPL-3.0-or-later

use super::provider::Provider;
use super::types::{
    AwamUri, BackendInfo, DirListing, ProviderCapability, ResolveResult, StatResult,
};

const LIST_CAP: &[ProviderCapability] = &[ProviderCapability::List];

/// Day-1 interface-only slots (ADR 0003) ? registered but not implemented.
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
            description: "qBittorrent / download task tree (interface-only; not Day-1)",
        }
    }

    pub fn prowlarr() -> Self {
        Self {
            id: "prowlarr",
            name: "Prowlarr",
            description: "Indexer search (interface-only; not Day-1)",
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
            detail: Some("Interface-only stub (ADR 0003); not implemented in Day-1".to_string()),
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
