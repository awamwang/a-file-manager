// SPDX-License-Identifier: GPL-3.0-or-later
//! ProviderRegistry — awam extension layer (ADR 0001 + 0003).
//! Day-1: rclone (CLI, no mount) + knowledge. No `awam://local` Kernel browse.
//!
//! AIA-76: depth-1 listing cache + `provider_preload` (warm on app start).

mod browser;
mod cache;
mod factory;
mod knowledge;
mod provider;
mod rclone;
mod registry;
mod stubs;
mod types;
mod uri;
mod util;

use cache::{depth1_cache_key, ListingCache};
use factory::build_day1_registry;
use registry::ProviderRegistry;
use serde::Serialize;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use types::{BackendInfo, DirListing, ResolveResult, StatResult};
use uri::parse_awam_uri;

static REGISTRY: LazyLock<ProviderRegistry> = LazyLock::new(build_day1_registry);
static LISTING_CACHE: LazyLock<Mutex<ListingCache>> =
    LazyLock::new(|| Mutex::new(ListingCache::new()));

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreloadReport {
    pub uris: Vec<String>,
    pub cached: usize,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn provider_backends() -> Result<Vec<BackendInfo>, String> {
    REGISTRY.backends()
}

#[tauri::command]
pub fn provider_list(uri: String) -> Result<DirListing, String> {
    list_with_cache(&REGISTRY, &LISTING_CACHE, &uri)
}

#[tauri::command]
pub fn provider_stat(uri: String) -> Result<StatResult, String> {
    REGISTRY.stat(&uri)
}

#[tauri::command]
pub fn provider_resolve(uri: String) -> Result<ResolveResult, String> {
    REGISTRY.resolve(&uri)
}

/// Preload depth-1 listings: `awam://root` + each non-interface-only backend root.
/// Wired from the frontend on **app start** (see `src/state/providerCache.ts`).
#[tauri::command]
pub fn provider_preload() -> Result<PreloadReport, String> {
    preload_depth1(&REGISTRY, &LISTING_CACHE)
}

/// Invalidate depth-1 cache. Omit `uri` (or pass empty) to clear all.
#[tauri::command]
pub fn provider_cache_invalidate(uri: Option<String>) -> Result<usize, String> {
    let mut cache = LISTING_CACHE.lock().map_err(|err| err.to_string())?;
    match uri.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => {
            let n = cache.len();
            cache.invalidate_all();
            Ok(n)
        }
        Some(raw) => {
            let parsed = parse_awam_uri(raw)?;
            let key = depth1_cache_key(&parsed)
                .ok_or_else(|| format!("Not a depth-1 URI (nothing to invalidate): {raw}"))?;
            Ok(usize::from(cache.invalidate_key(&key)))
        }
    }
}

fn list_with_cache(
    registry: &ProviderRegistry,
    cache_lock: &Mutex<ListingCache>,
    uri_str: &str,
) -> Result<DirListing, String> {
    let parsed = parse_awam_uri(uri_str)?;
    let key = depth1_cache_key(&parsed);
    if let Some(ref key) = key {
        if let Ok(mut cache) = cache_lock.lock() {
            if let Some(hit) = cache.get(key, Instant::now()) {
                return Ok(hit);
            }
        }
    }

    let listing = registry.list(uri_str)?;

    if let Some(key) = key {
        if let Ok(mut cache) = cache_lock.lock() {
            cache.put(key, listing.clone(), Instant::now());
        }
    }
    Ok(listing)
}

fn preload_depth1(
    registry: &ProviderRegistry,
    cache_lock: &Mutex<ListingCache>,
) -> Result<PreloadReport, String> {
    let mut uris = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();

    // Always warm the root roster first.
    match list_with_cache(registry, cache_lock, "awam://root") {
        Ok(listing) => {
            uris.push(listing.uri);
            if let Some(err) = listing.error {
                errors.push(format!("awam://root: {err}"));
            }
        }
        Err(err) => errors.push(format!("awam://root: {err}")),
    }

    let backends = registry.backends()?;
    for backend in backends {
        let target = format!("awam://{}", backend.id);
        if backend.interface_only {
            skipped.push(format!("{target} (interface-only)"));
            continue;
        }
        if !backend
            .capabilities
            .iter()
            .any(|cap| cap.eq_ignore_ascii_case("list"))
        {
            skipped.push(format!("{target} (no list capability)"));
            continue;
        }
        match list_with_cache(registry, cache_lock, &target) {
            Ok(listing) => {
                uris.push(listing.uri.clone());
                if let Some(err) = listing.error {
                    errors.push(format!("{target}: {err}"));
                }
            }
            Err(err) => errors.push(format!("{target}: {err}")),
        }
    }

    let cached = cache_lock
        .lock()
        .map(|cache| cache.len())
        .unwrap_or(0);

    Ok(PreloadReport {
        uris,
        cached,
        skipped,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::{AwamUri, ProviderCapability, ProviderEntry};
    use crate::providers::provider::Provider;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingProvider {
        id: &'static str,
        hits: Arc<AtomicUsize>,
    }

    impl Provider for CountingProvider {
        fn backend_id(&self) -> &str {
            self.id
        }
        fn name(&self) -> &str {
            self.id
        }
        fn description(&self) -> &str {
            "counting"
        }
        fn capabilities(&self) -> &[ProviderCapability] {
            &[ProviderCapability::List]
        }
        fn health(&self) -> BackendInfo {
            BackendInfo {
                id: self.id.to_string(),
                name: self.id.to_string(),
                description: "counting".to_string(),
                healthy: true,
                capabilities: vec!["list".to_string()],
                detail: None,
                interface_only: false,
            }
        }
        fn list(&self, uri: &AwamUri) -> DirListing {
            self.hits.fetch_add(1, Ordering::SeqCst);
            DirListing {
                uri: uri.to_string_uri(),
                backend: self.id.to_string(),
                entries: vec![ProviderEntry {
                    name: "child".to_string(),
                    uri: uri.child("child").to_string_uri(),
                    is_dir: true,
                    is_file: false,
                    size: 0,
                    modified_time: 0,
                    local_path: None,
                    extra: HashMap::new(),
                }],
                error: None,
            }
        }
    }

    struct StubOnly {
        id: &'static str,
    }

    impl Provider for StubOnly {
        fn backend_id(&self) -> &str {
            self.id
        }
        fn name(&self) -> &str {
            self.id
        }
        fn description(&self) -> &str {
            "stub"
        }
        fn capabilities(&self) -> &[ProviderCapability] {
            &[ProviderCapability::List]
        }
        fn interface_only(&self) -> bool {
            true
        }
        fn health(&self) -> BackendInfo {
            BackendInfo {
                id: self.id.to_string(),
                name: self.id.to_string(),
                description: "stub".to_string(),
                healthy: false,
                capabilities: vec!["list".to_string()],
                detail: Some("stub".to_string()),
                interface_only: true,
            }
        }
        fn list(&self, uri: &AwamUri) -> DirListing {
            DirListing {
                uri: uri.to_string_uri(),
                backend: self.id.to_string(),
                entries: Vec::new(),
                error: Some("interface-only".to_string()),
            }
        }
    }

    #[test]
    fn depth1_list_is_cached_deeper_is_not() {
        let hits = Arc::new(AtomicUsize::new(0));
        let registry = ProviderRegistry::new();
        registry
            .register(Arc::new(CountingProvider {
                id: "rclone",
                hits: hits.clone(),
            }))
            .unwrap();
        let cache = Mutex::new(ListingCache::new());

        let first = list_with_cache(&registry, &cache, "awam://rclone").unwrap();
        let second = list_with_cache(&registry, &cache, "awam://rclone").unwrap();
        assert_eq!(first, second);
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        let _ = list_with_cache(&registry, &cache, "awam://rclone/remote").unwrap();
        let _ = list_with_cache(&registry, &cache, "awam://rclone/remote").unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn preload_skips_interface_only_and_fills_cache() {
        let hits = Arc::new(AtomicUsize::new(0));
        let registry = ProviderRegistry::new();
        registry
            .register(Arc::new(CountingProvider {
                id: "knowledge",
                hits: hits.clone(),
            }))
            .unwrap();
        registry
            .register(Arc::new(StubOnly { id: "downloads" }))
            .unwrap();
        let cache = Mutex::new(ListingCache::new());

        let report = preload_depth1(&registry, &cache).unwrap();
        assert!(report.uris.iter().any(|u| u == "awam://root"));
        assert!(report.uris.iter().any(|u| u.contains("knowledge")));
        assert!(report
            .skipped
            .iter()
            .any(|s| s.contains("downloads") && s.contains("interface-only")));
        assert!(report.cached >= 2);
        // root does not call CountingProvider; knowledge listed once
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        // Second list hits cache
        let _ = list_with_cache(&registry, &cache, "awam://knowledge").unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }
}
