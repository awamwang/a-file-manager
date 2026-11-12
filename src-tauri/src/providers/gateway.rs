// SPDX-License-Identifier: GPL-3.0-or-later
//! Live HTTP client for awam-resource-gateway (awam-file).
//!
//! Discovery is dynamic: AFM reads `/api/backends` and proxies list/stat/resolve
//! without hardcoding each backend id. `awam://local` is filtered out (Navigator).

use super::types::{
    is_hidden_provider_backend, BackendInfo, DirListing, ProviderEntry, QueryParam, ResolveResult,
    StatResult,
};
use super::uri::parse_awam_uri;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use ureq::Agent;

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:18787";
/// 设置项键：Power 层可配置网关地址。
pub const GATEWAY_URL_SETTING_KEY: &str = "power.awamGatewayUrl";
/// 存活探测：只打 `/health`，不跑各适配器健康检查。
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
/// `/api/backends` 与 list/stat/resolve 会触及适配器探测（qBittorrent/docker 等），
/// 本机实测常需数秒；过短会误判网关离线并回退到内置列表。
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
const PROBE_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub online: bool,
    pub base_url: String,
    pub detail: Option<String>,
    pub backend_count: Option<usize>,
}

#[derive(Debug, Clone)]
struct ProbeCache {
    checked_at: Instant,
    online: bool,
    detail: Option<String>,
    backend_count: Option<usize>,
}

static PROBE_CACHE: Mutex<Option<ProbeCache>> = Mutex::new(None);
/// 来自设置的网关地址（已规范化）；`None` 表示使用默认。
static CONFIGURED_URL: Mutex<Option<String>> = Mutex::new(None);

/// 规范化网关 base URL：去空白、去尾部 `/`；空串视为未配置。
pub fn normalize_gateway_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// 应用设置中的网关地址，并清空探测缓存以便立即重连。
pub fn set_configured_gateway_url(raw: Option<&str>) {
    let normalized = raw.and_then(normalize_gateway_url);
    if let Ok(mut guard) = CONFIGURED_URL.lock() {
        *guard = normalized;
    }
    invalidate_probe_cache();
}

/// 解析顺序：环境变量 → 设置 → 默认 `http://127.0.0.1:18787`。
pub fn gateway_base_url() -> String {
    if let Ok(env) =
        std::env::var("AWAM_GATEWAY_URL").or_else(|_| std::env::var("VITE_AWAM_GATEWAY_URL"))
    {
        if let Some(url) = normalize_gateway_url(&env) {
            return url;
        }
    }
    if let Ok(guard) = CONFIGURED_URL.lock() {
        if let Some(url) = guard.as_ref() {
            return url.clone();
        }
    }
    DEFAULT_BASE_URL.to_string()
}

fn agent(timeout: Duration) -> Agent {
    Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}

fn get_json(
    base_url: &str,
    path_and_query: &str,
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let url = format!("{base_url}{path_and_query}");
    let mut response = agent(timeout)
        .get(&url)
        .call()
        .map_err(|err| format!("awam-file gateway request failed ({url}): {err}"))?;
    response
        .body_mut()
        .read_json::<serde_json::Value>()
        .map_err(|err| format!("awam-file gateway JSON decode failed ({url}): {err}"))
}

/// Invalidate cached reachability (e.g. after a mid-session list failure).
pub fn invalidate_probe_cache() {
    if let Ok(mut guard) = PROBE_CACHE.lock() {
        *guard = None;
    }
}

pub fn status() -> GatewayStatus {
    let base_url = gateway_base_url();
    if let Ok(guard) = PROBE_CACHE.lock() {
        if let Some(cache) = guard.as_ref() {
            if cache.checked_at.elapsed() < PROBE_TTL {
                return GatewayStatus {
                    online: cache.online,
                    base_url,
                    detail: cache.detail.clone(),
                    backend_count: cache.backend_count,
                };
            }
        }
    }

    match probe_now(&base_url) {
        Ok(()) => {
            store_probe(true, None, None);
            GatewayStatus {
                online: true,
                base_url,
                detail: None,
                backend_count: None,
            }
        }
        Err(err) => {
            store_probe(false, Some(err.clone()), None);
            GatewayStatus {
                online: false,
                base_url,
                detail: Some(err),
                backend_count: None,
            }
        }
    }
}

pub fn is_online() -> bool {
    status().online
}

fn store_probe(online: bool, detail: Option<String>, backend_count: Option<usize>) {
    if let Ok(mut guard) = PROBE_CACHE.lock() {
        *guard = Some(ProbeCache {
            checked_at: Instant::now(),
            online,
            detail,
            backend_count,
        });
    }
}

fn probe_now(base_url: &str) -> Result<(), String> {
    // `/health` 轻量存活检查；后端数量由 fetch_backends / Providers 列表填充。
    let value = get_json(base_url, "/health", HEALTH_TIMEOUT)?;
    if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        return Err(format!(
            "awam-file gateway /health unexpected payload: {value}"
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct GatewayBackendsResponse {
    backends: Vec<GatewayBackendInfo>,
}

#[derive(Debug, Deserialize)]
struct GatewayBackendInfo {
    id: String,
    name: String,
    description: String,
    healthy: bool,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    detail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GatewayDirListing {
    uri: String,
    backend: String,
    #[serde(default)]
    entries: Vec<GatewayEntry>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default, alias = "requiredQuery")]
    required_query: Option<Vec<QueryParam>>,
}

#[derive(Debug, Deserialize)]
struct GatewayEntry {
    name: String,
    uri: String,
    is_dir: bool,
    is_file: bool,
    #[serde(default)]
    size: u64,
    /// Gateway uses unix seconds (float); AFM Providers use ms.
    #[serde(default)]
    modified_time: f64,
    #[serde(default)]
    local_path: Option<String>,
    #[serde(default)]
    extra: HashMap<String, serde_json::Value>,
    #[serde(default, alias = "requiredQuery")]
    required_query: Option<Vec<QueryParam>>,
}

#[derive(Debug, Deserialize)]
struct GatewayStatResult {
    #[serde(default)]
    entry: Option<GatewayEntry>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GatewayResolveResult {
    uri: String,
    #[serde(default)]
    local_path: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    action_args: HashMap<String, serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
}

fn parse_backends_json(value: &serde_json::Value) -> Result<Vec<GatewayBackendInfo>, String> {
    serde_json::from_value::<GatewayBackendsResponse>(value.clone())
        .map(|r| r.backends)
        .map_err(|err| format!("unexpected /api/backends payload: {err}"))
}

fn is_local_backend(id: &str) -> bool {
    id.eq_ignore_ascii_case("local")
}

/// 网关 list 条目：按 URI 后端过滤 `search`（不误伤 `awam://prowlarr/search` 或 `awam://searching`）。
fn exclude_gateway_list_entry(uri: &str, name: &str) -> bool {
    if uri.to_ascii_lowercase().starts_with("awam://local")
        || name.eq_ignore_ascii_case("local")
    {
        return true;
    }
    match parse_awam_uri(uri) {
        Ok(parsed) => is_hidden_provider_backend(&parsed.backend),
        Err(_) => false,
    }
}

fn map_backend(info: GatewayBackendInfo) -> BackendInfo {
    BackendInfo {
        id: info.id,
        name: info.name,
        description: info.description,
        healthy: info.healthy,
        capabilities: info.capabilities,
        detail: info.detail,
        interface_only: false,
        source: Some("gateway".to_string()),
    }
}

fn map_entry(entry: GatewayEntry) -> ProviderEntry {
    let modified_ms = if entry.modified_time > 0.0 {
        (entry.modified_time * 1000.0).round() as u64
    } else {
        0
    };
    ProviderEntry {
        name: entry.name,
        uri: entry.uri,
        is_dir: entry.is_dir,
        is_file: entry.is_file,
        size: entry.size,
        modified_time: modified_ms,
        local_path: entry.local_path,
        extra: entry.extra,
        required_query: entry.required_query,
    }
}

/// Discovered backends from the live gateway (excludes `local` and hidden `search`).
pub fn fetch_backends() -> Result<Vec<BackendInfo>, String> {
    let base_url = gateway_base_url();
    let value = get_json(&base_url, "/api/backends", HTTP_TIMEOUT).map_err(|err| {
        store_probe(false, Some(err.clone()), None);
        err
    })?;
    let backends = parse_backends_json(&value).map_err(|err| {
        store_probe(false, Some(err.clone()), None);
        err
    })?;
    let mapped: Vec<BackendInfo> = backends
        .into_iter()
        .filter(|b| !is_local_backend(&b.id) && !is_hidden_provider_backend(&b.id))
        .map(map_backend)
        .collect();
    store_probe(true, None, Some(mapped.len()));
    Ok(mapped)
}

fn encode_uri_query(uri: &str) -> String {
    // Minimal query escaping for awam:// and path characters.
    let mut out = String::with_capacity(uri.len() * 2);
    for ch in uri.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            _ => {
                for byte in ch.to_string().as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

pub fn list(uri: &str) -> Result<DirListing, String> {
    let base_url = gateway_base_url();
    let path = format!("/api/list?uri={}", encode_uri_query(uri));
    let value = get_json(&base_url, &path, HTTP_TIMEOUT).map_err(|err| {
        invalidate_probe_cache();
        err
    })?;
    let listing: GatewayDirListing = serde_json::from_value(value)
        .map_err(|err| format!("unexpected /api/list payload: {err}"))?;
    let entries = listing
        .entries
        .into_iter()
        .filter(|e| !exclude_gateway_list_entry(&e.uri, &e.name))
        .map(map_entry)
        .collect();
    Ok(DirListing {
        uri: listing.uri,
        backend: listing.backend,
        entries,
        error: listing.error,
        required_query: listing.required_query,
    })
}

pub fn stat(uri: &str) -> Result<StatResult, String> {
    let base_url = gateway_base_url();
    let path = format!("/api/stat?uri={}", encode_uri_query(uri));
    let value = get_json(&base_url, &path, HTTP_TIMEOUT).map_err(|err| {
        invalidate_probe_cache();
        err
    })?;
    let result: GatewayStatResult = serde_json::from_value(value)
        .map_err(|err| format!("unexpected /api/stat payload: {err}"))?;
    Ok(StatResult {
        entry: result.entry.map(map_entry),
        error: result.error,
    })
}

pub fn resolve(uri: &str) -> Result<ResolveResult, String> {
    let base_url = gateway_base_url();
    let path = format!("/api/resolve?uri={}", encode_uri_query(uri));
    let value = get_json(&base_url, &path, HTTP_TIMEOUT).map_err(|err| {
        invalidate_probe_cache();
        err
    })?;
    let result: GatewayResolveResult = serde_json::from_value(value)
        .map_err(|err| format!("unexpected /api/resolve payload: {err}"))?;
    Ok(ResolveResult {
        uri: result.uri,
        local_path: result.local_path,
        action: result.action,
        action_args: result.action_args,
        error: result.error,
    })
}

/// Pure helpers for unit tests (no network).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_gateway_url_trims_and_strips_slash() {
        assert_eq!(
            normalize_gateway_url("  http://127.0.0.1:18787/  ").as_deref(),
            Some("http://127.0.0.1:18787")
        );
        assert_eq!(normalize_gateway_url("   "), None);
        assert_eq!(normalize_gateway_url(""), None);
    }

    #[test]
    fn configured_url_overrides_default_when_no_env() {
        // 不依赖环境变量；测设置缓存路径。
        let prev = std::env::var_os("AWAM_GATEWAY_URL");
        let prev_vite = std::env::var_os("VITE_AWAM_GATEWAY_URL");
        std::env::remove_var("AWAM_GATEWAY_URL");
        std::env::remove_var("VITE_AWAM_GATEWAY_URL");

        set_configured_gateway_url(Some("http://10.0.0.2:9999/"));
        assert_eq!(gateway_base_url(), "http://10.0.0.2:9999");
        set_configured_gateway_url(Some(""));
        assert_eq!(gateway_base_url(), DEFAULT_BASE_URL);

        match prev {
            Some(v) => std::env::set_var("AWAM_GATEWAY_URL", v),
            None => std::env::remove_var("AWAM_GATEWAY_URL"),
        }
        match prev_vite {
            Some(v) => std::env::set_var("VITE_AWAM_GATEWAY_URL", v),
            None => std::env::remove_var("VITE_AWAM_GATEWAY_URL"),
        }
    }

    #[test]
    fn filters_local_from_backends_payload() {
        let value = serde_json::json!({
            "backends": [
                {
                    "id": "local",
                    "name": "Local",
                    "description": "disk",
                    "healthy": true,
                    "capabilities": ["list"],
                    "detail": null
                },
                {
                    "id": "smb",
                    "name": "SMB",
                    "description": "shares",
                    "healthy": false,
                    "capabilities": ["list", "stat"],
                    "detail": "no roots"
                },
                {
                    "id": "hf",
                    "name": "Hugging Face",
                    "description": "cache",
                    "healthy": true,
                    "capabilities": ["list"],
                    "detail": null
                }
            ]
        });
        let backends = parse_backends_json(&value).unwrap();
        let mapped: Vec<BackendInfo> = backends
            .into_iter()
            .filter(|b| !is_local_backend(&b.id))
            .map(map_backend)
            .collect();
        assert_eq!(mapped.len(), 2);
        assert!(mapped
            .iter()
            .all(|b| b.source.as_deref() == Some("gateway")));
        assert!(mapped.iter().all(|b| b.id != "local"));
        assert!(mapped.iter().any(|b| b.id == "smb" && !b.healthy));
        assert!(mapped.iter().any(|b| b.id == "hf" && b.healthy));
    }

    #[test]
    fn maps_modified_time_seconds_to_ms() {
        let entry = map_entry(GatewayEntry {
            name: "page.md".to_string(),
            uri: "awam://knowledge/page.md".to_string(),
            is_dir: false,
            is_file: true,
            size: 12,
            modified_time: 1_700_000_000.5,
            local_path: None,
            extra: HashMap::new(),
            required_query: None,
        });
        assert_eq!(entry.modified_time, 1_700_000_000_500);
    }

    #[test]
    fn encode_uri_keeps_awam_scheme_safe() {
        let encoded = encode_uri_query("awam://smb/share/a b");
        assert!(encoded.contains("awam"));
        assert!(encoded.contains("%20") || encoded.contains("%3A"));
    }

    #[test]
    fn filters_search_from_backends_payload() {
        let value = serde_json::json!({
            "backends": [
                {"id":"search","name":"Everything search","description":"x","healthy":true,"capabilities":["list"]},
                {"id":"rclone","name":"rclone","description":"x","healthy":true,"capabilities":["list"]}
            ]
        });
        let backends = parse_backends_json(&value).unwrap();
        let mapped: Vec<_> = backends
            .into_iter()
            .filter(|b| !is_local_backend(&b.id) && !is_hidden_provider_backend(&b.id))
            .map(map_backend)
            .collect();
        assert!(mapped.iter().all(|b| b.id != "search"));
        assert!(mapped.iter().any(|b| b.id == "rclone"));
    }

    #[test]
    fn filters_search_from_backends_payload_case_insensitively() {
        let value = serde_json::json!({
            "backends": [
                {"id":"SEARCH","name":"Everything","description":"x","healthy":true,"capabilities":["list"]},
                {"id":"hf","name":"Hugging Face","description":"x","healthy":true,"capabilities":["list"]}
            ]
        });
        let backends = parse_backends_json(&value).unwrap();
        let mapped: Vec<_> = backends
            .into_iter()
            .filter(|b| !is_local_backend(&b.id) && !is_hidden_provider_backend(&b.id))
            .map(map_backend)
            .collect();
        assert!(mapped.iter().all(|b| !b.id.eq_ignore_ascii_case("search")));
        assert!(mapped.iter().any(|b| b.id == "hf"));
    }

    #[test]
    fn list_entry_filter_uri_backend_not_display_name() {
        assert!(
            !exclude_gateway_list_entry("awam://prowlarr/search", "search"),
            "prowlarr 路径段 search 不应因展示名被隐藏"
        );
        assert!(
            exclude_gateway_list_entry("awam://search/everything", "everything"),
            "awam://search 后端条目应被过滤"
        );
        assert!(
            !exclude_gateway_list_entry("awam://searching/item", "item"),
            "awam://searching 不得误匹配 search 前缀"
        );
    }

    #[test]
    fn parses_required_query_from_list_payload() {
        let value = serde_json::json!({
            "uri": "awam://hf/remote",
            "backend": "hf",
            "entries": [{
                "name": "remote",
                "uri": "awam://hf/remote",
                "is_dir": true,
                "is_file": false,
                "requiredQuery": [{"key": "q"}]
            }],
            "error": null,
            "requiredQuery": [{"key": "q"}]
        });
        let listing: GatewayDirListing = serde_json::from_value(value).unwrap();
        let listing_params = listing.required_query.expect("listing requiredQuery");
        assert_eq!(listing_params[0].key, "q");
        assert!(listing_params[0].required);
        let entry_params = listing.entries[0]
            .required_query
            .as_ref()
            .expect("entry requiredQuery");
        assert_eq!(entry_params[0].key, "q");
        assert!(entry_params[0].required);
    }
}
