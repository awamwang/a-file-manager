// SPDX-License-Identifier: GPL-3.0-or-later

#[cfg(feature = "ai-addon")]
mod ai;
mod archive;
mod autostart;
mod deps_probe;
mod discovery;
mod extension_host;
mod favorites;
mod visit_stats;
mod file_ops;
mod navigator;
mod perf_log;
mod paths;
mod platform;
mod preview;
mod preview_bridge;
mod shell_icons;
mod ide_launch;
mod open_with;
mod projects;
mod providers;
mod settings;
mod shell_menu;
mod system_tray;
mod terminal;
mod tool_runtime;
pub use tool_runtime::{
    config_file_path, ensure_gateway, health_check, load_from_path, query_gateway_status,
    read_http_token, resolve_http_token, resolve_runtime_dir, save_to_path, start_sidecar,
    version_in_range, EnsureGatewayResult, GatewayStatus, HealthOk, ToolRuntimeConfig,
    CONFIG_FILE_NAME, DEFAULT_BASE_URL, RUNTIME_DIR_ENV, TOKEN_ENV_VAR, TOKEN_FILE_NAME,
    TOOL_RUNTIME_PACK_ID,
};
mod snap_chrome;
#[cfg(windows)]
mod drag_focus;
#[cfg(windows)]
mod window_icon;
mod win_process;
mod work_groups;

fn launched_from_autostart(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == autostart::AFM_AUTOSTART_CLI_FLAG)
}

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthPayload {
    pub ok: bool,
    pub name: String,
    pub version: String,
    pub identifier: String,
    /// True when this binary was built with Cargo feature `ai-addon` (AIA-222).
    pub ai_addon: bool,
}

#[tauri::command]
fn health_get() -> HealthPayload {
    HealthPayload {
        ok: true,
        name: "A File Manager".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        identifier: "com.a-file-manager.app".to_string(),
        ai_addon: cfg!(feature = "ai-addon"),
    }
}

/// Quit the process (tray Quit + close-confirm Quit share this path — AIA-86).
#[tauri::command]
fn app_quit(app: tauri::AppHandle) {
    system_tray::destroy_system_tray(&app);
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;
    use tauri_plugin_frame::{FramePluginBuilder, WebviewWindowExt};

    let raw_args: Vec<String> = std::env::args().collect();
    let from_autostart = launched_from_autostart(&raw_args);
    let add_favorite_path = favorites::take_add_favorite_path(&raw_args);

    // GUI single-instance: avoid stacked tray icons from a second process.
    // Shell-verb "add favorite" is allowed through so it can still mutate favorites.
    let primary_instance = system_tray::try_acquire_single_instance_lock();
    if !primary_instance && add_favorite_path.is_none() {
        eprintln!("A File Manager is already running (single-instance); exiting.");
        return;
    }

    // AIA-271: AUMID before any HWND so taskbar grouping matches NSIS shortcuts.
    #[cfg(windows)]
    window_icon::set_process_app_user_model_id();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Native OLE/file drag-out (CF_HDROP) for dropping onto players / Explorer / etc.
        // NOTE: on macOS this plugin's `start_dragging` command aborts the process (a `RecvError`
        // from an `unwrap` inside the plugin); the frontend never calls it on macOS — see
        // src/state/dnd/dragOut.ts and src/views/NavigatorView.vue (which use the WebView-native
        // HTML5 file drag-out via text/uri-list instead).
        .plugin(tauri_plugin_drag::init())
        .plugin(
            FramePluginBuilder::new()
                // Keep Fluent self-drawn caption; plugin only installs HTMAXBUTTON overlay.
                .auto_titlebar(false)
                .snap_overlay(true)
                .titlebar_height(snap_chrome::TITLEBAR_HEIGHT_PX)
                .button_width(snap_chrome::BUTTON_WIDTH_PX)
                .build(),
        )
        .setup(move |app| {
            // Taskbar icon: Tauri set_icon only sets ICON_SMALL; also apply PE ICON_BIG (AIA-271).
            let app_icon = system_tray::load_app_icon(app.handle());
            if let Some(window) = app.get_webview_window("main") {
                window.create_overlay_titlebar_with_height(snap_chrome::TITLEBAR_HEIGHT_PX)?;
                if let Some(icon) = app_icon.clone() {
                    if let Err(err) = window.set_icon(icon) {
                        eprintln!("window set_icon failed: {err}");
                    }
                }
            }
            #[cfg(windows)]
            window_icon::apply_main_window_icons(app.handle());
            // AIA-261: Explorer-like — background mousedown for outbound drag must not raise AFM.
            #[cfg(windows)]
            drag_focus::install_for_main_window(app.handle());

            // 7.16: window chrome is reported by the platform adapter instead of
            // a scattered `#[cfg(target_os = "macos")]` branch. Windows returns
            // `FluentSelfDrawn` and leaves `decorations:false` + the Fluent caption
            // alone; macOS requests native decorations + a transparent Overlay
            // titlebar so traffic lights float over the self-drawn caption (7.14
            // option C). `decorations:false` in tauri.conf.json stays untouched
            // (hard assertion in check-window-controls.mjs).
            if crate::platform::window_chrome() == crate::platform::WindowChrome::MacOverlay {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_decorations(true);
                    let _ = window.set_title("");
                    // tao rewrites the whole styleMask for `set_decorations(true)`
                    // on a dispatch_async main-queue task, dropping the
                    // FullSizeContentView bit. Queue the Overlay switch behind
                    // that task, then restore the bit in the adapter (7.18):
                    // without it AppKit keeps contentView 28pt shorter than the
                    // window and the caption starts below an empty grey band.
                    let overlay_app = app.handle().clone();
                    let _ = window.run_on_main_thread(move || {
                        if let Some(window) = overlay_app.get_webview_window("main") {
                            let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
                            crate::platform::extend_content_under_titlebar(&window);
                        }
                    });
                    // AppKit re-lays out contentView on resize / after reveal;
                    // re-apply on the main thread after every size change and
                    // once at startup for windows revealed without a resize.
                    let chrome_app = app.handle().clone();
                    window.on_window_event(move |event| {
                        if matches!(event, tauri::WindowEvent::Resized(_)) {
                            if let Some(window) = chrome_app.get_webview_window("main") {
                                let main_thread_app = chrome_app.clone();
                                let _ = window.run_on_main_thread(move || {
                                    if let Some(window) =
                                        main_thread_app.get_webview_window("main")
                                    {
                                        crate::platform::extend_content_under_titlebar(&window);
                                    }
                                });
                            }
                        }
                    });
                    // Startup fallback: a window revealed without a resize never
                    // fires Resized. The adapter call is idempotent.
                    let late_app = app.handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1600));
                        let align_app = late_app.clone();
                        let _ = late_app.run_on_main_thread(move || {
                            if let Some(window) = align_app.get_webview_window("main") {
                                crate::platform::extend_content_under_titlebar(&window);
                            }
                        });
                    });
                }
            }

            // Only the primary instance owns the tray icon.
            if primary_instance {
                if let Err(err) = system_tray::setup_system_tray(app.handle()) {
                    eprintln!("system tray setup failed: {err}");
                }
            }

            // AIA-272: window starts `visible: false`. Reveal after frontend paint unless
            // OS login autostart + `core.launchAtStartupHidden` (stay in tray until restore).
            let hide_on_autostart =
                from_autostart && settings::read_bool_setting(app.handle(), "core.launchAtStartupHidden");
            app.manage(system_tray::StartupReveal::new(!hide_on_autostart));

            // Discovery blacklist is unused until the user searches — keep it off
            // the setup critical path (extra settings disk read + service mutate).
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    discovery::sync_blacklist_from_settings(&handle);
                });
            }

            // 加载 awam-file 网关地址设置，供 Providers 探测使用。
            {
                let url = settings::read_string_setting(
                    app.handle(),
                    providers::gateway::GATEWAY_URL_SETTING_KEY,
                );
                providers::gateway::set_configured_gateway_url(Some(url.as_str()));
            }

            // 从 APPDATA 加载本机 Feature Pack。
            match app.path().app_config_dir() {
                Ok(dir) => {
                    if let Err(err) = extension_host::load_from_disk(&dir) {
                        eprintln!("extension host load_from_disk failed: {err}");
                    }
                }
                Err(err) => {
                    eprintln!("extension host: failed to resolve app config dir: {err}");
                }
            }

            if let Some(path) = add_favorite_path.clone() {
                match favorites::apply_add_favorite_cli(app.handle(), &path) {
                    Ok(added) => eprintln!(
                        "AFM favorites shell verb: {} ({path})",
                        if added { "added" } else { "already present" }
                    ),
                    Err(err) => eprintln!("AFM favorites shell verb failed: {err}"),
                }
                // Secondary process was only needed for the shell verb — exit after applying.
                if !primary_instance {
                    app.handle().exit(0);
                }
            }

            Ok(())
        })
        .on_menu_event(|app, event| {
            system_tray::handle_menu_event(app, event);
        })
        .invoke_handler(tauri::generate_handler![
                    health_get,
                    platform::platform_capabilities,
                    platform::platform_home_dir,
                    deps_probe::deps_probe_get,
                    app_quit,
                    system_tray::afm_startup_should_reveal,
                    system_tray::afm_reveal_main_window,
                    autostart::autostart_status,
                    autostart::autostart_set_enabled,
                    navigator::navigator_list,
                    navigator::navigator_search,
                    navigator::navigator_default_path,
                    navigator::navigator_parent,
                    navigator::navigator_stat,
                    navigator::navigator_get_attributes,
                    navigator::navigator_set_attributes,
                    navigator::navigator_list_drives,
                    navigator::navigator_user_folders,
                    navigator::navigator_shell_places,
                    navigator::navigator_disk_space,
                    navigator::navigator_expand_path,
                    ide_launch::ide_list_apps,
                    ide_launch::ide_open_path,
                    file_ops::file_ops_name_exists,
                    file_ops::file_ops_copy,
                    file_ops::file_ops_move,
                    file_ops::file_ops_rename,
                    file_ops::file_ops_create_dir,
                    file_ops::file_ops_create_file,
                    file_ops::shortcut::file_ops_create_shortcuts,
                    file_ops::file_ops_delete,
                    file_ops::file_ops_delete_permanent,
                    file_ops::file_ops_can_recycle,
                    file_ops::file_ops_recycle_list,
                    file_ops::file_ops_restore,
                    file_ops::file_ops_new_templates,
                    file_ops::clipboard::file_ops_clipboard_set,
                    file_ops::clipboard::file_ops_clipboard_get,
                    file_ops::clipboard::file_ops_clipboard_clear,
                    #[cfg(windows)]
                    drag_focus::dnd_arm_outbound_paths,
                    #[cfg(windows)]
                    drag_focus::dnd_clear_outbound_paths,
                    shell_icons::shell_icon_get,
                    shell_icons::shell_icon_get_many,
                    settings::settings_schema_get,
                    settings::settings_get,
                    settings::settings_set,
                    discovery::discovery_status,
                    discovery::discovery_estimate_count,
                    discovery::discovery_set_fallback_enabled,
                    discovery::discovery_search,
                    discovery::discovery_set_blacklist,
                    providers::provider_backends,
                    providers::provider_gateway_status,
                    tool_runtime::tool_runtime_get_config,
                    tool_runtime::tool_runtime_set_config,
                    tool_runtime::tool_runtime_gateway_status,
                    tool_runtime::tool_runtime_ensure_gateway,
                    tool_runtime::tool_runtime_list_tools,
                    tool_runtime::tool_runtime_list_capabilities,
                    tool_runtime::tool_runtime_call_capability,
                    providers::provider_list,
                    providers::provider_stat,
                    providers::provider_resolve,
                    providers::provider_preload,
                    providers::provider_cache_invalidate,
                    work_groups::work_groups_get,
                    work_groups::work_groups_create,
                    work_groups::work_groups_add_member,
                    work_groups::work_groups_remove_member,
                    work_groups::work_groups_rename,
                    work_groups::work_groups_copy,
                    work_groups::work_groups_delete,
                    work_groups::work_groups_silent_switch,
                    work_groups::work_groups_save_open_state,
                    work_groups::work_groups_save_members_from_open_state,
                    work_groups::work_groups_reorder,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_addon_status,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_agent_targets_list,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_agent_targets_discover,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_agent_targets_set_enabled,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_agent_targets_set_default_in_projects,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_agent_targets_update,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_agent_targets_upsert_custom,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_agent_targets_remove,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skill_deploy_mode_set,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skills_library_get,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skills_library_set_root,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skills_import,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skills_remove,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skills_deploy,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skills_undeploy,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_skills_search_project,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_mcp_catalog_list,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_mcp_catalog_upsert,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_mcp_catalog_remove,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_mcp_catalog_import,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_mcp_deploy,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_mcp_undeploy,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_harvest_mcp_scan,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_harvest_mcp_import,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_harvest_skills_scan,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_harvest_skills_import,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_project_ai_get,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_project_ai_set_enabled_agents,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_project_deployments,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_harness_init,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_providers_list,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_providers_set_enabled,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_providers_update,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_providers_upsert_compatible,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_providers_remove,
                    #[cfg(feature = "ai-addon")]
                    ai::ai_project_open_editor,
                    projects::projects_index_get,
                    projects::projects_index_set_active,
                    projects::projects_root_ensure,
                    projects::projects_workspace_create,
                    projects::projects_workspace_list,
                    projects::projects_workspace_reorder,
                    projects::projects_workspace_bind_work_group,
                    projects::projects_workspace_activate,
                    projects::projects_workspace_member_add,
                    projects::projects_workspace_member_list,
                    projects::projects_workspace_member_remove,
                    projects::projects_resolve_open_path,
                    projects::projects_workspace_context_get,
                    projects::projects_workspace_context_set,
                    projects::projects_workspace_board_get,
                    projects::projects_workspace_board_add_task,
                    projects::projects_workspace_board_update_task,
                    projects::projects_workspace_board_move_task,
                    projects::projects_workspace_board_delete_task,
                    projects::projects_workspace_delete,
                    projects::projects_workspace_rename,
                    projects::projects_workspace_copy,
                    projects::projects_workspace_promote_from_work_group,
                    extension_host::extension_host_status,
                    extension_host::extension_host_marketplace,
                    extension_host::extension_host_list_installed,
                    extension_host::extension_host_install,
                    extension_host::extension_host_set_enabled,
                    extension_host::extension_host_uninstall,
                    extension_host::extension_host_dispatch_ui_command,
                    extension_host::extension_host_list_fs_provider_hooks,
                    extension_host::extension_host_register_fs_provider_hook,
                    extension_host::extension_host_install_pack,
                    extension_host::extension_host_list_packs,
                    extension_host::extension_host_set_pack_enabled,
                    extension_host::extension_host_uninstall_pack,
                    extension_host::extension_host_list_contributions,
                    shell_menu::shell_show_context_menu,
                    shell_menu::shell_get_context_menu,
                    shell_menu::shell_invoke_context_menu_item,
                    terminal::terminal_list_apps,
                    terminal::open_in_terminal,
                    favorites::favorites_get,
                    favorites::favorites_toggle,
                    favorites::favorites_contains,
                    favorites::favorites_os_hook_status,
                    favorites::favorites_os_hook_set_enabled,
                    visit_stats::visit_stats_get,
                    visit_stats::visit_stats_record,
                    visit_stats::visit_stats_quick_access,
                    open_with::open_path_default,
                    open_with::open_recycle_bin_system,
                    open_with::open_native_open_with_dialog,
                    preview::preview_load,
                    preview_bridge::preview_bridge_invoke,
                    preview_bridge::preview_bridge_switch,
                    preview_bridge::preview_bridge_dismiss,
                    archive::archive_compress_zip,
                    archive::archive_extract_zip
                ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Drop the NotifyIcon on graceful exit so `tauri dev` / HMR does not stack ghosts.
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                system_tray::destroy_system_tray(app_handle);
            }
        });
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
        assert_eq!(health.ai_addon, cfg!(feature = "ai-addon"));
    }
}
