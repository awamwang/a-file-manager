// SPDX-License-Identifier: GPL-3.0-or-later

use super::provider::Provider;
use super::types::{AwamUri, BackendInfo, DirListing, ProviderEntry, ResolveResult, StatResult};
use super::uri::parse_awam_uri;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Dynamic provider registry (extension layer). Kernel OS browse stays on NavigatorService.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: RwLock<HashMap<String, Arc<dyn Provider>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, provider: Arc<dyn Provider>) -> Result<(), String> {
        let id = provider.backend_id().to_ascii_lowercase();
        if id == "local" {
            return Err(
                "Refusing to register 'local' as an extension provider; Kernel browse uses NavigatorService (ADR 0001/0003)"
                    .to_string(),
            );
        }
        if id == "root" {
            return Err("backend id 'root' is reserved for the registry root listing".to_string());
        }
        let mut map = self.providers.write().map_err(|err| err.to_string())?;
        if map.contains_key(&id) {
            return Err(format!("Provider already registered: {id}"));
        }
        map.insert(id, provider);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn unregister(&self, backend_id: &str) -> Result<bool, String> {
        let id = backend_id.to_ascii_lowercase();
        let mut map = self.providers.write().map_err(|err| err.to_string())?;
        Ok(map.remove(&id).is_some())
    }

    #[allow(dead_code)]
    pub fn contains(&self, backend_id: &str) -> bool {
        let id = backend_id.to_ascii_lowercase();
        self.providers
            .read()
            .map(|map| map.contains_key(&id))
            .unwrap_or(false)
    }

    pub fn backends(&self) -> Result<Vec<BackendInfo>, String> {
        let map = self.providers.read().map_err(|err| err.to_string())?;
        let mut infos: Vec<BackendInfo> = map.values().map(|provider| provider.health()).collect();
        infos.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(infos)
    }

    pub fn list(&self, uri_str: &str) -> Result<DirListing, String> {
        let uri = parse_awam_uri(uri_str)?;
        if uri.backend == "root" {
            return self.list_root(&uri);
        }
        if uri.backend == "local" {
            return Ok(DirListing {
                uri: uri.to_string_uri(),
                backend: "local".to_string(),
                entries: Vec::new(),
                error: Some(
                    "awam://local is not a Day-1 extension backend; use NavigatorService for OS paths (ADR 0001/0003)"
                        .to_string(),
                ),
            });
        }
        let provider = self.get_provider(&uri.backend)?;
        Ok(provider.list(&uri))
    }

    pub fn stat(&self, uri_str: &str) -> Result<StatResult, String> {
        let uri = parse_awam_uri(uri_str)?;
        if uri.backend == "root" {
            return Ok(StatResult {
                entry: Some(ProviderEntry {
                    name: "root".to_string(),
                    uri: "awam://root".to_string(),
                    is_dir: true,
                    is_file: false,
                    size: 0,
                    modified_time: 0,
                    local_path: None,
                    extra: HashMap::new(),
                }),
                error: None,
            });
        }
        let provider = self.get_provider(&uri.backend)?;
        Ok(provider.stat(&uri))
    }

    pub fn resolve(&self, uri_str: &str) -> Result<ResolveResult, String> {
        let uri = parse_awam_uri(uri_str)?;
        if uri.backend == "root" {
            return Ok(ResolveResult {
                uri: uri.to_string_uri(),
                local_path: None,
                action: None,
                action_args: HashMap::new(),
                error: Some("root URI has no local path".to_string()),
            });
        }
        let provider = self.get_provider(&uri.backend)?;
        Ok(provider.resolve(&uri))
    }

    fn get_provider(&self, backend_id: &str) -> Result<Arc<dyn Provider>, String> {
        let id = backend_id.to_ascii_lowercase();
        let map = self.providers.read().map_err(|err| err.to_string())?;
        map.get(&id)
            .cloned()
            .ok_or_else(|| format!("Unknown provider backend: {id}"))
    }

    fn list_root(&self, uri: &AwamUri) -> Result<DirListing, String> {
        let backends = self.backends()?;
        let entries = backends
            .into_iter()
            .map(|info| {
                let mut extra = HashMap::new();
                extra.insert(
                    "healthy".to_string(),
                    serde_json::Value::Bool(info.healthy),
                );
                if info.interface_only {
                    extra.insert("interfaceOnly".to_string(), serde_json::Value::Bool(true));
                }
                if let Some(detail) = info.detail.clone() {
                    extra.insert("detail".to_string(), serde_json::Value::String(detail));
                }
                ProviderEntry {
                    name: info.id.clone(),
                    uri: format!("awam://{}", info.id),
                    is_dir: true,
                    is_file: false,
                    size: 0,
                    modified_time: 0,
                    local_path: None,
                    extra,
                }
            })
            .collect();
        Ok(DirListing {
            uri: uri.to_string_uri(),
            backend: "root".to_string(),
            entries,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::ProviderCapability;

    struct DummyProvider {
        id: &'static str,
    }

    impl Provider for DummyProvider {
        fn backend_id(&self) -> &str {
            self.id
        }
        fn name(&self) -> &str {
            self.id
        }
        fn description(&self) -> &str {
            "dummy"
        }
        fn capabilities(&self) -> &[ProviderCapability] {
            &[ProviderCapability::List]
        }
        fn health(&self) -> BackendInfo {
            BackendInfo {
                id: self.id.to_string(),
                name: self.id.to_string(),
                description: "dummy".to_string(),
                healthy: true,
                capabilities: vec!["list".to_string()],
                detail: None,
                interface_only: false,
            }
        }
        fn list(&self, uri: &AwamUri) -> DirListing {
            DirListing {
                uri: uri.to_string_uri(),
                backend: self.id.to_string(),
                entries: Vec::new(),
                error: None,
            }
        }
    }

    #[test]
    fn dynamic_register_and_root_list() {
        let registry = ProviderRegistry::new();
        registry
            .register(Arc::new(DummyProvider { id: "rclone" }))
            .unwrap();
        registry
            .register(Arc::new(DummyProvider { id: "knowledge" }))
            .unwrap();
        let root = registry.list("awam://root/").unwrap();
        assert_eq!(root.entries.len(), 2);
        assert!(root.entries.iter().any(|e| e.name == "knowledge"));
        assert!(root.entries.iter().any(|e| e.name == "rclone"));
    }

    #[test]
    fn refuses_local_provider() {
        let registry = ProviderRegistry::new();
        let err = registry
            .register(Arc::new(DummyProvider { id: "local" }))
            .unwrap_err();
        assert!(err.contains("NavigatorService"));
    }

    #[test]
    fn awam_local_list_is_rejected() {
        let registry = ProviderRegistry::new();
        let listing = registry.list("awam://local/G:/temp").unwrap();
        assert!(listing.error.as_ref().unwrap().contains("NavigatorService"));
    }
}
