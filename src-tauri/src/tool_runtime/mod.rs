// SPDX-License-Identifier: GPL-3.0-or-later
//! 工具管理运行时：Pack 私有配置、网关附着探活、sidecar 拉起与 MCP 调用封装。

mod config;
mod gateway;
mod mcp_client;
mod semver_compat;
mod sidecar;

pub use config::{
    config_file_path, load_from_path, save_to_path, ToolRuntimeConfig, CONFIG_FILE_NAME,
    DEFAULT_BASE_URL, TOOL_RUNTIME_PACK_ID,
};
pub use gateway::{
    health_check, query_gateway_status, read_http_token, resolve_http_token, resolve_runtime_dir,
    GatewayStatus, HealthOk, RUNTIME_DIR_ENV, TOKEN_ENV_VAR, TOKEN_FILE_NAME,
};
pub use semver_compat::version_in_range;
pub use sidecar::{ensure_gateway, start_sidecar, EnsureGatewayResult};

use crate::extension_host::types::{ExtensionRuntimeState, GatewayCompat, InstalledPackRecord};
use serde_json::Value;
use std::path::Path;
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

/// 优先附着已有网关，否则按 Pack sidecar 贡献拉起进程。
#[tauri::command]
pub async fn tool_runtime_ensure_gateway(app: AppHandle) -> Result<EnsureGatewayResult, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("tool_runtime_ensure_gateway: app config dir: {err}"))?;
    tauri::async_runtime::spawn_blocking(move || sidecar::ensure_gateway(&dir))
        .await
        .map_err(|err| format!("tool_runtime_ensure_gateway worker failed: {err}"))?
}

/// 列出网关登记的工具（可选 `kind`）。
#[tauri::command]
pub async fn tool_runtime_list_tools(
    app: AppHandle,
    kind: Option<String>,
) -> Result<Value, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        with_mcp_client(&dir, |client| client.list_tools(kind.as_deref()))
    })
    .await
    .map_err(|err| format!("tool_runtime_list_tools worker failed: {err}"))?
}

/// 列出指定工具的 capability（`refresh` 时向网关强制刷新）。
#[tauri::command]
pub async fn tool_runtime_list_capabilities(
    app: AppHandle,
    tool_id: String,
    refresh: Option<bool>,
) -> Result<Value, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        with_mcp_client(&dir, |client| client.list_capabilities(&tool_id, refresh))
    })
    .await
    .map_err(|err| format!("tool_runtime_list_capabilities worker failed: {err}"))?
}

/// 调用指定工具的 capability。
#[tauri::command]
pub async fn tool_runtime_call_capability(
    app: AppHandle,
    tool_id: String,
    capability_id: String,
    arguments: Option<Value>,
    execute: Option<bool>,
    allow_destructive: Option<bool>,
) -> Result<Value, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        with_mcp_client(&dir, |client| {
            client.call_capability(
                &tool_id,
                &capability_id,
                arguments.as_ref(),
                execute,
                allow_destructive,
            )
        })
    })
    .await
    .map_err(|err| format!("tool_runtime_call_capability worker failed: {err}"))?
}

fn app_config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("tool_runtime: app config dir: {err}"))
}

/// Pack 未启用则拒绝；否则 `ensure_gateway`、校验 gatewayCompat，再执行 MCP 调用。
fn with_mcp_client<R, F>(app_data: &Path, f: F) -> Result<R, String>
where
    F: FnOnce(&mut mcp_client::McpClient) -> Result<R, String>,
{
    require_pack_enabled(sidecar::find_tool_runtime_pack().as_ref())?;
    sidecar::ensure_gateway(app_data)?;
    let base_url = gateway::load_base_url(app_data);
    let health_version = gateway::health_check(&base_url)
        .ok()
        .and_then(|ok| ok.version);
    let token = gateway::resolve_http_token(&gateway::resolve_runtime_dir())?;
    let mut client = mcp_client::McpClient::connect(&base_url, &token)?;
    let version = health_version.or_else(|| client.server_version().map(str::to_string));
    check_gateway_compat(version.as_deref())?;
    f(&mut client)
}

fn check_gateway_compat(version: Option<&str>) -> Result<(), String> {
    let compat = sidecar::find_tool_runtime_pack().and_then(|pack| {
        crate::extension_host::pack_fs::read_pack_json(Path::new(&pack.install_path))
            .ok()
            .and_then(|manifest| manifest.gateway_compat)
    });
    assert_gateway_compat(version, compat.as_ref())
}

pub(crate) fn require_pack_enabled(pack: Option<&InstalledPackRecord>) -> Result<(), String> {
    match pack {
        Some(pack) if pack.state == ExtensionRuntimeState::Enabled => Ok(()),
        _ => Err("tool_runtime_disabled".to_string()),
    }
}

pub(crate) fn assert_gateway_compat(
    version: Option<&str>,
    compat: Option<&GatewayCompat>,
) -> Result<(), String> {
    let Some(version) = version.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let Some(compat) = compat else {
        return Ok(());
    };
    if !semver_compat::known_triple(version) {
        return Ok(());
    }
    if version_in_range(version, &compat.min_version, &compat.max_version) {
        Ok(())
    } else {
        Err(format!(
            "gatewayCompat mismatch: version {version} not in [{}, {}]",
            compat.min_version, compat.max_version
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension_host::types::ExtensionPermission;

    fn sample_pack(state: ExtensionRuntimeState) -> InstalledPackRecord {
        InstalledPackRecord {
            id: TOOL_RUNTIME_PACK_ID.to_string(),
            name: "工具管理运行时".to_string(),
            version: "0.1.0".to_string(),
            state,
            permissions: vec![ExtensionPermission::Network],
            install_path: "/tmp/pack".to_string(),
            last_error: None,
        }
    }

    #[test]
    fn pack_not_enabled_is_disabled() {
        assert_eq!(
            require_pack_enabled(None).unwrap_err(),
            "tool_runtime_disabled"
        );
        assert_eq!(
            require_pack_enabled(Some(&sample_pack(ExtensionRuntimeState::Disabled))).unwrap_err(),
            "tool_runtime_disabled"
        );
        assert!(require_pack_enabled(Some(&sample_pack(ExtensionRuntimeState::Enabled))).is_ok());
    }

    #[test]
    fn gateway_compat_skips_unknown_version() {
        let compat = GatewayCompat {
            min_version: "0.1.0".to_string(),
            max_version: "1.0.0".to_string(),
        };
        assert!(assert_gateway_compat(None, Some(&compat)).is_ok());
        assert!(assert_gateway_compat(Some("dev"), Some(&compat)).is_ok());
        assert!(assert_gateway_compat(Some("0.9.0"), Some(&compat)).is_ok());
        let err = assert_gateway_compat(Some("2.0.0"), Some(&compat)).unwrap_err();
        assert!(err.contains("gatewayCompat mismatch"), "{err}");
        assert!(err.contains("2.0.0"), "{err}");
    }
}
