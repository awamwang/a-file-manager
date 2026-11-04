// SPDX-License-Identifier: GPL-3.0-or-later
//! Discovery — Everything IPC first-class (ADR 0002). No `es.exe` product path.

mod backend;
mod everything_ipc;
mod fallback;
#[cfg(test)]
mod mock;
mod service;
mod types;

use everything_ipc::WindowsEverythingIpc;
use service::DiscoveryService;
use std::sync::{Arc, LazyLock};
use types::{DiscoverySearchOptions, DiscoverySearchResult, DiscoveryStatus};

static DISCOVERY: LazyLock<DiscoveryService> = LazyLock::new(|| {
    // Fallback off by default — no dual-default with self-index.
    DiscoveryService::new(Arc::new(WindowsEverythingIpc), false)
});

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
