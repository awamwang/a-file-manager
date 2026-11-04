// SPDX-License-Identifier: GPL-3.0-or-later

mod discovery;
mod navigator;
mod projects;
mod providers;
mod settings;
mod work_groups;

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
            providers::provider_backends,
            providers::provider_list,
            providers::provider_stat,
            providers::provider_resolve,
            work_groups::work_groups_get,
            work_groups::work_groups_create,
            work_groups::work_groups_add_member,
            work_groups::work_groups_silent_switch,
            work_groups::work_groups_save_open_state,
            work_groups::work_groups_save_members_from_open_state,
            projects::projects_get,
            projects::projects_create,
            projects::projects_rename,
            projects::projects_delete,
            projects::projects_set_active,
            projects::projects_add_workspace_link,
            projects::projects_remove_workspace_link,
            projects::projects_set_context_notes,
            projects::projects_add_task,
            projects::projects_update_task,
            projects::projects_move_task,
            projects::projects_delete_task,
            projects::projects_open_all_links,
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
