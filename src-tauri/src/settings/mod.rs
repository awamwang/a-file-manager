// SPDX-License-Identifier: GPL-3.0-or-later
//! Settings schema v1 — Rust authority (Core / Power / Experimental).

mod defaults;
mod migrate;
mod sanitize;
mod store;
mod types;

use defaults::schema_manifest;
use serde_json::Value;
use std::collections::BTreeMap;
use store::{load_from_path, set_values_at_path};
use tauri::{AppHandle, Manager};
use types::{SettingsDocument, SettingsSchemaManifest};

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

/// Read a boolean setting from disk (defaults to `false` on missing/error).
pub fn read_bool_setting(app: &AppHandle, key: &str) -> bool {
    read_bool_setting_or(app, key, false)
}

/// Read a boolean setting; use `default` when missing or unloadable.
pub fn read_bool_setting_or(app: &AppHandle, key: &str, default: bool) -> bool {
    let Ok(dir) = config_dir(app) else {
        return default;
    };
    let Ok(document) = load_from_path(&dir) else {
        return default;
    };
    document
        .values
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(default)
}

/// Read a string setting from disk (empty on missing/error).
pub fn read_string_setting(app: &AppHandle, key: &str) -> String {
    let Ok(dir) = config_dir(app) else {
        return String::new();
    };
    let Ok(document) = load_from_path(&dir) else {
        return String::new();
    };
    document
        .values
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

/// Load settings document from a config directory (used by Discovery blacklist sync).
pub fn store_load_for_discovery(config_dir: &std::path::Path) -> Result<SettingsDocument, String> {
    load_from_path(config_dir)
}

#[tauri::command]
pub fn settings_schema_get() -> SettingsSchemaManifest {
    schema_manifest()
}

#[tauri::command]
pub fn settings_get(app: AppHandle) -> Result<SettingsDocument, String> {
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

#[tauri::command]
pub fn settings_set(
    app: AppHandle,
    patch: BTreeMap<String, Value>,
) -> Result<SettingsDocument, String> {
    let dir = config_dir(&app)?;
    let sync_blacklist = patch.contains_key("core.discoveryBlacklist");
    let sync_projects_root = patch.contains_key("core.projectsRoot");
    let sync_gateway_url = patch.contains_key(crate::providers::gateway::GATEWAY_URL_SETTING_KEY);
    let gateway_url_patch = patch
        .get(crate::providers::gateway::GATEWAY_URL_SETTING_KEY)
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let document = set_values_at_path(&dir, patch)?;
    if sync_blacklist {
        crate::discovery::sync_blacklist_from_settings(&app);
    }
    if sync_projects_root {
        // Keep thin projects-index.json Root in sync with the setting (AIA-207 / C1).
        let _ = crate::projects::sync_index_root_from_settings(&app);
    }
    if sync_gateway_url {
        crate::providers::gateway::set_configured_gateway_url(gateway_url_patch.as_deref());
        // 网关地址变更后清空发现/列表会话缓存，下次进入 Providers 时重新探测。
        crate::providers::invalidate_provider_session();
    }
    Ok(document)
}
