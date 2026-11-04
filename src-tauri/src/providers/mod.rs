// SPDX-License-Identifier: GPL-3.0-or-later
//! ProviderRegistry — awam extension layer (ADR 0001 + 0003).
//! Day-1: rclone (CLI, no mount) + knowledge. No `awam://local` Kernel browse.

mod factory;
mod knowledge;
mod provider;
mod rclone;
mod registry;
mod stubs;
mod types;
mod uri;
mod util;

use factory::build_day1_registry;
use registry::ProviderRegistry;
use std::sync::LazyLock;
use types::{BackendInfo, DirListing, ResolveResult, StatResult};

static REGISTRY: LazyLock<ProviderRegistry> = LazyLock::new(build_day1_registry);

#[tauri::command]
pub fn provider_backends() -> Result<Vec<BackendInfo>, String> {
    REGISTRY.backends()
}

#[tauri::command]
pub fn provider_list(uri: String) -> Result<DirListing, String> {
    REGISTRY.list(&uri)
}

#[tauri::command]
pub fn provider_stat(uri: String) -> Result<StatResult, String> {
    REGISTRY.stat(&uri)
}

#[tauri::command]
pub fn provider_resolve(uri: String) -> Result<ResolveResult, String> {
    REGISTRY.resolve(&uri)
}
