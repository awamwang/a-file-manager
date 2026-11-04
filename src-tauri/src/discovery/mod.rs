// SPDX-License-Identifier: GPL-3.0-or-later
//! Discovery — Everything IPC first-class (ADR 0002). No `es.exe` product path.

mod backend;
mod blacklist;
mod everything_ipc;
mod fallback;
#[cfg(test)]
mod mock;
mod service;
mod types;

use everything_ipc::WindowsEverythingIpc;
use service::DiscoveryService;
use std::sync::{Arc, LazyLock};
use tauri::{AppHandle, Manager};
use types::{DiscoverySearchOptions, DiscoverySearchResult, DiscoveryStatus};

static DISCOVERY: LazyLock<DiscoveryService> = LazyLock::new(|| {
    // Fallback off by default — no dual-default with self-index.
    DiscoveryService::new(Arc::new(WindowsEverythingIpc), false)
});

/// Apply persisted `core.discoveryBlacklist` into the live Discovery service (AIA-61).
pub fn sync_blacklist_from_settings(app: &AppHandle) {
    let Ok(dir) = app.path().app_config_dir() else {
        return;
    };
    let Ok(document) = crate::settings::store_load_for_discovery(&dir) else {
        return;
    };
    let entries = document
        .values
        .get("core.discoveryBlacklist")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    DISCOVERY.set_blacklist(entries);
}

#[tauri::command]
pub fn discovery_status() -> DiscoveryStatus {
    DISCOVERY.status()
}

#[tauri::command]
pub fn discovery_set_fallback_enabled(enabled: bool) -> DiscoveryStatus {
    DISCOVERY.set_fallback_enabled(enabled);
    DISCOVERY.status()
}

#[tauri::command]
pub fn discovery_search(
    query: String,
    options: Option<DiscoverySearchOptions>,
) -> Result<DiscoverySearchResult, String> {
    DISCOVERY.search(query, options)
}

#[tauri::command]
pub fn discovery_set_blacklist(
    app: AppHandle,
    paths: Vec<String>,
) -> Result<DiscoveryStatus, String> {
    let cleaned = DISCOVERY.set_blacklist(paths);
    // Persist alongside other Core settings.
    let mut patch = std::collections::BTreeMap::new();
    patch.insert(
        "core.discoveryBlacklist".to_string(),
        serde_json::Value::Array(
            cleaned
                .iter()
                .map(|path| serde_json::Value::String(path.clone()))
                .collect(),
        ),
    );
    crate::settings::settings_set(app, patch)?;
    Ok(DISCOVERY.status())
}
