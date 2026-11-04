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
    let Ok(dir) = config_dir(app) else {
        return false;
    };
    let Ok(document) = load_from_path(&dir) else {
        return false;
    };
    document
        .values
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
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
    set_values_at_path(&dir, patch)
}
