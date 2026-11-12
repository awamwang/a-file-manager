// SPDX-License-Identifier: GPL-3.0-or-later
//! ProviderRegistry — awam extension layer (ADR 0001 + 0003).
//! 内置：rclone（CLI，不挂载）+ browser。不含 `awam://local`。
//! knowledge / downloads / prowlarr 等由在线网关动态发现。
//!
//! AIA-76: depth-1 listing cache + `provider_preload` (warm on app start).
//! AIA-226: prefer live awam-file gateway discovery (`/api/backends`); fall back to
//! builtin registry when the gateway is offline.
//! 后端发现结果亦做会话级缓存，避免每次进入 Providers / 路径栏访问协议根时重复打
//! `/api/backends`；刷新或改网关地址时主动失效。

mod browser;
mod cache;
mod factory;
pub(crate) mod gateway;
/// 网关离线时不再注册；保留实现供网关在线代理与后续扩展。
#[allow(dead_code)]
mod knowledge;
mod provider;
pub(crate) mod rclone;
mod registry;
/// 占位提供方；内置列表不再展示，保留类型定义。
#[allow(dead_code)]
mod stubs;
mod types;
mod uri;
mod util;

use cache::{depth1_cache_key, ListingCache};
use factory::build_builtin_registry;
use registry::ProviderRegistry;
use serde::Serialize;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use types::{BackendInfo, DirListing, ResolveResult, StatResult};
use uri::parse_awam_uri;

static REGISTRY: LazyLock<ProviderRegistry> = LazyLock::new(build_builtin_registry);
static LISTING_CACHE: LazyLock<Mutex<ListingCache>> =
    LazyLock::new(|| Mutex::new(ListingCache::new()));

/// `/api/backends` 探测较慢；会话内复用，直到刷新或网关地址变更。
const BACKENDS_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone)]
struct BackendsCacheEntry {
    backends: Vec<BackendInfo>,
    fetched_at: Instant,
}

static BACKENDS_CACHE: LazyLock<Mutex<Option<BackendsCacheEntry>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreloadReport {
    pub uris: Vec<String>,
    pub cached: usize,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

fn builtin_backends() -> Result<Vec<BackendInfo>, String> {
    let mut backends = REGISTRY.backends()?;
    for backend in &mut backends {
        if backend.source.is_none() {
            backend.source = Some("builtin".to_string());
        }
    }
    Ok(backends)
}

fn local_rejected_listing(uri: &str) -> DirListing {
    DirListing {
        uri: uri.to_string(),
        backend: "local".to_string(),
        entries: Vec::new(),
        error: Some(
            "awam://local is not an AFM extension backend; use Navigator for OS paths (ADR 0001/0003)"
                .to_string(),
        ),
    }
}

/// 清空后端发现缓存（改网关地址 / 手动刷新时调用）。
pub fn invalidate_backends_cache() {
    if let Ok(mut guard) = BACKENDS_CACHE.lock() {
        *guard = None;
    }
}

/// 清空探测、后端发现与 depth-1 列表缓存。
pub fn invalidate_provider_session() {
    gateway::invalidate_probe_cache();
    invalidate_backends_cache();
    if let Ok(mut cache) = LISTING_CACHE.lock() {
        cache.invalidate_all();
    }
}

/// Prefer gateway discovery; keep builtins when gateway is down.
/// 命中会话缓存时直接返回，避免重复请求 `/api/backends`。
fn discover_backends() -> Result<Vec<BackendInfo>, String> {
    if let Ok(guard) = BACKENDS_CACHE.lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.fetched_at.elapsed() < BACKENDS_TTL {
                return Ok(entry.backends.clone());
            }
        }
    }

    let backends = match gateway::fetch_backends() {
        Ok(list) => list,
        Err(_) => builtin_backends()?,
    };
    if let Ok(mut guard) = BACKENDS_CACHE.lock() {
        *guard = Some(BackendsCacheEntry {
            backends: backends.clone(),
            fetched_at: Instant::now(),
        });
    }
    Ok(backends)
}

fn list_dispatch(uri_str: &str) -> Result<DirListing, String> {
    let parsed = parse_awam_uri(uri_str)?;
    if parsed.backend == "local" {
        return Ok(local_rejected_listing(&parsed.to_string_uri()));
    }
    if gateway::is_online() {
        match gateway::list(uri_str) {
            Ok(listing) => Ok(listing),
            Err(err) => {
                // Mid-flight outage: degrade to builtin for known ids / root.
                if parsed.backend == "root"
                    || REGISTRY.contains(&parsed.backend)
                {
                    REGISTRY.list(uri_str)
                } else {
                    Err(err)
                }
            }
        }
    } else {
        REGISTRY.list(uri_str)
    }
}

fn stat_dispatch(uri_str: &str) -> Result<StatResult, String> {
    let parsed = parse_awam_uri(uri_str)?;
    if parsed.backend == "local" {
        return Ok(StatResult {
            entry: None,
            error: Some(
                "awam://local is not an AFM extension backend; use Navigator for OS paths (ADR 0001/0003)"
                    .to_string(),
            ),
        });
    }
    if gateway::is_online() {
        match gateway::stat(uri_str) {
            Ok(result) => Ok(result),
            Err(_) if parsed.backend == "root" || REGISTRY.contains(&parsed.backend) => {
                REGISTRY.stat(uri_str)
            }
            Err(err) => Err(err),
        }
    } else {
        REGISTRY.stat(uri_str)
    }
}

fn resolve_dispatch(uri_str: &str) -> Result<ResolveResult, String> {
    let parsed = parse_awam_uri(uri_str)?;
    if parsed.backend == "local" {
        return Ok(ResolveResult {
            uri: parsed.to_string_uri(),
            local_path: None,
            action: None,
            action_args: Default::default(),
            error: Some(
                "awam://local is not an AFM extension backend; use Navigator for OS paths (ADR 0001/0003)"
                    .to_string(),
            ),
        });
    }
    if gateway::is_online() {
        match gateway::resolve(uri_str) {
            Ok(result) => Ok(result),
            Err(_) if parsed.backend == "root" || REGISTRY.contains(&parsed.backend) => {
                REGISTRY.resolve(uri_str)
            }
            Err(err) => Err(err),
        }
    } else {
        REGISTRY.resolve(uri_str)
    }
}

/// Provider IPC 一律 `spawn_blocking`：rclone SSH / knowledge 扫盘等不得卡住 UI 线程。
#[tauri::command]
pub async fn provider_gateway_status() -> gateway::GatewayStatus {
    tauri::async_runtime::spawn_blocking(gateway::status)
        .await
        .unwrap_or_else(|_| gateway::GatewayStatus {
            online: false,
            base_url: gateway::gateway_base_url(),
            detail: Some("provider_gateway_status worker failed".to_string()),
            backend_count: None,
        })
}

#[tauri::command]
pub async fn provider_backends() -> Result<Vec<BackendInfo>, String> {
    tauri::async_runtime::spawn_blocking(discover_backends)
        .await
        .map_err(|err| format!("provider_backends worker failed: {err}"))?
}

#[tauri::command]
pub async fn provider_list(uri: String) -> Result<DirListing, String> {
    tauri::async_runtime::spawn_blocking(move || list_with_cache(&LISTING_CACHE, &uri))
        .await
        .map_err(|err| format!("provider_list worker failed: {err}"))?
}

#[tauri::command]
pub async fn provider_stat(uri: String) -> Result<StatResult, String> {
    tauri::async_runtime::spawn_blocking(move || stat_dispatch(&uri))
        .await
        .map_err(|err| format!("provider_stat worker failed: {err}"))?
}

#[tauri::command]
pub async fn provider_resolve(uri: String) -> Result<ResolveResult, String> {
    tauri::async_runtime::spawn_blocking(move || resolve_dispatch(&uri))
        .await
        .map_err(|err| format!("provider_resolve worker failed: {err}"))?
}

/// Preload depth-1 listings: `awam://root` + each non-interface-only backend root.
/// Wired from the frontend on idle / Providers visit (see `src/state/providerCache.ts`).
/// Respects `power.providerPreloadKnowledge` (AIA-182).
#[tauri::command]
pub async fn provider_preload(app: AppHandle) -> Result<PreloadReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let include_knowledge =
            crate::settings::read_bool_setting_or(&app, "power.providerPreloadKnowledge", true);
        preload_depth1(&LISTING_CACHE, include_knowledge)
    })
    .await
    .map_err(|err| format!("provider_preload worker failed: {err}"))?
}

/// Invalidate depth-1 cache. Omit `uri` (or pass empty) to clear all（同时清后端发现缓存）。
#[tauri::command]
pub async fn provider_cache_invalidate(uri: Option<String>) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        match uri.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => {
                let n = LISTING_CACHE
                    .lock()
                    .map(|cache| cache.len())
                    .unwrap_or(0);
                invalidate_provider_session();
                Ok(n)
            }
            Some(raw) => {
                let parsed = parse_awam_uri(raw)?;
                let key = depth1_cache_key(&parsed)
                    .ok_or_else(|| format!("Not a depth-1 URI (nothing to invalidate): {raw}"))?;
                let mut cache = LISTING_CACHE.lock().map_err(|err| err.to_string())?;
                Ok(usize::from(cache.invalidate_key(&key)))
            }
        }
    })
    .await
    .map_err(|err| format!("provider_cache_invalidate worker failed: {err}"))?
}

fn list_with_cache(
    cache_lock: &Mutex<ListingCache>,
    uri_str: &str,
) -> Result<DirListing, String> {
    list_with_cache_using(cache_lock, uri_str, list_dispatch)
}

fn list_with_cache_using<F>(
    cache_lock: &Mutex<ListingCache>,
    uri_str: &str,
    fetch: F,
) -> Result<DirListing, String>
where
    F: Fn(&str) -> Result<DirListing, String>,
{
    let parsed = parse_awam_uri(uri_str)?;
    let key = depth1_cache_key(&parsed);
    if let Some(ref key) = key {
        if let Ok(mut cache) = cache_lock.lock() {
            if let Some(hit) = cache.get(key, Instant::now()) {
                return Ok(hit);
            }
        }
    }

    let listing = fetch(uri_str)?;

    if let Some(key) = key {
        if let Ok(mut cache) = cache_lock.lock() {
            cache.put(key, listing.clone(), Instant::now());
        }
    }
    Ok(listing)
}

fn preload_depth1(
    cache_lock: &Mutex<ListingCache>,
    include_knowledge: bool,
) -> Result<PreloadReport, String> {
    preload_depth1_using(cache_lock, include_knowledge, discover_backends, list_dispatch)
}

fn preload_depth1_using<BF, LF>(
    cache_lock: &Mutex<ListingCache>,
    include_knowledge: bool,
    backends_fn: BF,
    list_fn: LF,
) -> Result<PreloadReport, String>
where
    BF: FnOnce() -> Result<Vec<BackendInfo>, String>,
    LF: Fn(&str) -> Result<DirListing, String>,
{
    let mut uris = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();

    // Always warm the root roster first.
    match list_with_cache_using(cache_lock, "awam://root", &list_fn) {
        Ok(listing) => {
            uris.push(listing.uri);
            if let Some(err) = listing.error {
                errors.push(format!("awam://root: {err}"));
            }
        }
        Err(err) => errors.push(format!("awam://root: {err}")),
    }

    let backends = backends_fn()?;
    for backend in backends {
        let target = format!("awam://{}", backend.id);
        if backend.interface_only {
            skipped.push(format!("{target} (interface-only)"));
            continue;
        }
        if backend.id.eq_ignore_ascii_case("knowledge") && !include_knowledge {
            skipped.push(format!("{target} (preload disabled by settings)"));
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
        match list_with_cache_using(cache_lock, &target, &list_fn) {
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
                source: Some("builtin".to_string()),
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
                source: Some("builtin".to_string()),
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
        let fetch = |uri: &str| registry.list(uri);

        let first = list_with_cache_using(&cache, "awam://rclone", fetch).unwrap();
        let second = list_with_cache_using(&cache, "awam://rclone", fetch).unwrap();
        assert_eq!(first, second);
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        let _ = list_with_cache_using(&cache, "awam://rclone/remote", fetch).unwrap();
        let _ = list_with_cache_using(&cache, "awam://rclone/remote", fetch).unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn preload_skips_interface_only_and_fills_cache() {
        let hits = Arc::new(AtomicUsize::new(0));
        let registry = ProviderRegistry::new();
        registry
            .register(Arc::new(CountingProvider {
                id: "rclone",
                hits: hits.clone(),
            }))
            .unwrap();
        registry
            .register(Arc::new(StubOnly { id: "stub-only" }))
            .unwrap();
        let cache = Mutex::new(ListingCache::new());
        let list_fn = |uri: &str| registry.list(uri);
        let backends_fn = || registry.backends();

        let report = preload_depth1_using(&cache, true, backends_fn, list_fn).unwrap();
        assert!(report.uris.iter().any(|u| u == "awam://root"));
        assert!(report.uris.iter().any(|u| u.contains("rclone")));
        assert!(report
            .skipped
            .iter()
            .any(|s| s.contains("stub-only") && s.contains("interface-only")));
        assert!(report.cached >= 2);
        // root does not call CountingProvider; rclone listed once
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        // Second list hits cache
        let _ = list_with_cache_using(&cache, "awam://rclone", list_fn).unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn preload_can_skip_knowledge_when_disabled() {
        let hits = Arc::new(AtomicUsize::new(0));
        let registry = ProviderRegistry::new();
        registry
            .register(Arc::new(CountingProvider {
                id: "knowledge",
                hits: hits.clone(),
            }))
            .unwrap();
        let cache = Mutex::new(ListingCache::new());
        let list_fn = |uri: &str| registry.list(uri);
        let backends_fn = || registry.backends();

        let report = preload_depth1_using(&cache, false, backends_fn, list_fn).unwrap();
        assert!(report.uris.iter().any(|u| u == "awam://root"));
        assert!(!report.uris.iter().any(|u| u.contains("knowledge")));
        assert!(report
            .skipped
            .iter()
            .any(|s| s.contains("knowledge") && s.contains("preload disabled")));
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn local_uri_stays_rejected_when_dispatching() {
        let listing = list_dispatch("awam://local/G:/temp").unwrap();
        assert!(listing
            .error
            .as_ref()
            .unwrap()
            .contains("Navigator"));
    }
}
