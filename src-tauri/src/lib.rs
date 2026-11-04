// SPDX-License-Identifier: GPL-3.0-or-later

mod discovery;
mod navigator;
mod settings;

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthPayload {
    pub ok: bool,
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[tauri::command]
fn health_get() -> HealthPayload {
    HealthPayload {
        ok: true,
        name: "A File Manager".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        identifier: "com.a-file-manager.app".to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            health_get,
            navigator::navigator_list,
            navigator::navigator_default_path,
            navigator::navigator_parent,
            settings::settings_schema_get,
            settings::settings_get,
            settings::settings_set,
            discovery::discovery_status,
            discovery::discovery_set_fallback_enabled,
            discovery::discovery_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_get_returns_ok_payload() {
        let health = health_get();
        assert!(health.ok);
        assert_eq!(health.name, "A File Manager");
        assert_eq!(health.identifier, "com.a-file-manager.app");
        assert!(!health.version.is_empty());
    }
}
