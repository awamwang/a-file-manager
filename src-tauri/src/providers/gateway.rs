// SPDX-License-Identifier: GPL-3.0-or-later
//! Live HTTP client for awam-resource-gateway (awam-file).
//!
//! Discovery is dynamic: AFM reads `/api/backends` and proxies list/stat/resolve
//! without hardcoding each backend id. `awam://local` is filtered out (Navigator).

use super::types::{BackendInfo, DirListing, ProviderEntry, ResolveResult, StatResult};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use ureq::Agent;

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:18787";
const HTTP_TIMEOUT: Duration = Duration::from_millis(800);
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

pub fn gateway_base_url() -> String {
    std::env::var("AWAM_GATEWAY_URL")
        .or_else(|_| std::env::var("VITE_AWAM_GATEWAY_URL"))
        .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .build()
        .into()
}

fn get_json(base_url: &str, path_and_query: &str) -> Result<serde_json::Value, String> {
    let url = format!("{base_url}{path_and_query}");
    let mut response = agent()
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
        Ok(count) => {
            store_probe(true, None, Some(count));
            GatewayStatus {
                online: true,
                base_url,
                detail: None,
                backend_count: Some(count),
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

fn probe_now(base_url: &str) -> Result<usize, String> {
    // Prefer /api/backends so a healthy process with a broken health route still works,
    // and we learn the discovered backend count in one round-trip.
    let value = get_json(base_url, "/api/backends")?;
    let backends = parse_backends_json(&value)?;
    Ok(backends.len())
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
    }
}

/// Discovered backends from the live gateway (excludes `local`).
pub fn fetch_backends() -> Result<Vec<BackendInfo>, String> {
    let base_url = gateway_base_url();
    let value = get_json(&base_url, "/api/backends").map_err(|err| {
        store_probe(false, Some(err.clone()), None);
        err
    })?;
    let backends = parse_backends_json(&value).map_err(|err| {
        store_probe(false, Some(err.clone()), None);
        err
    })?;
    let mapped: Vec<BackendInfo> = backends
        .into_iter()
        .filter(|b| !is_local_backend(&b.id))
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
    let value = get_json(&base_url, &path).map_err(|err| {
        invalidate_probe_cache();
        err
    })?;
    let listing: GatewayDirListing = serde_json::from_value(value)
        .map_err(|err| format!("unexpected /api/list payload: {err}"))?;
    let entries = listing
        .entries
        .into_iter()
        .filter(|e| {
            // Root roster may include awam://local — keep Navigator-only.
            !e.uri.to_ascii_lowercase().starts_with("awam://local")
                && !e.name.eq_ignore_ascii_case("local")
        })
        .map(map_entry)
        .collect();
    Ok(DirListing {
        uri: listing.uri,
        backend: listing.backend,
        entries,
        error: listing.error,
    })
}

pub fn stat(uri: &str) -> Result<StatResult, String> {
    let base_url = gateway_base_url();
    let path = format!("/api/stat?uri={}", encode_uri_query(uri));
    let value = get_json(&base_url, &path).map_err(|err| {
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
    let value = get_json(&base_url, &path).map_err(|err| {
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
        assert!(mapped.iter().all(|b| b.source.as_deref() == Some("gateway")));
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
        });
        assert_eq!(entry.modified_time, 1_700_000_000_500);
    }

    #[test]
    fn encode_uri_keeps_awam_scheme_safe() {
        let encoded = encode_uri_query("awam://smb/share/a b");
        assert!(encoded.contains("awam"));
        assert!(encoded.contains("%20") || encoded.contains("%3A"));
    }
}
