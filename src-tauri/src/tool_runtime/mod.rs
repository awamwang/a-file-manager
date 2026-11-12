// SPDX-License-Identifier: GPL-3.0-or-later
//! 工具管理运行时：Pack 私有配置、网关附着探活与版本兼容校验。

mod config;
mod gateway;
mod semver_compat;

pub use config::{
    config_file_path, load_from_path, save_to_path, ToolRuntimeConfig, CONFIG_FILE_NAME,
    DEFAULT_BASE_URL, TOOL_RUNTIME_PACK_ID,
};
pub use gateway::{
    health_check, query_gateway_status, read_http_token, resolve_runtime_dir, GatewayStatus,
    HealthOk, RUNTIME_DIR_ENV, TOKEN_FILE_NAME,
};
pub use semver_compat::version_in_range;

use tauri::{AppHandle, Manager};

/// 查询工具管理网关附着状态（网络 I/O 在 blocking 线程）。
#[tauri::command]
pub async fn tool_runtime_gateway_status(app: AppHandle) -> GatewayStatus {
    let dir = app.path().app_config_dir().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let base_url = dir
            .as_deref()
            .map(gateway::load_base_url)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        query_gateway_status(&base_url)
    })
    .await
    .unwrap_or_else(|_| GatewayStatus {
        reachable: false,
        detail: "tool_runtime_gateway_status worker failed".to_string(),
        version: None,
    })
}
