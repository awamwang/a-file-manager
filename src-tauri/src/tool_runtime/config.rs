// SPDX-License-Identifier: GPL-3.0-or-later

use crate::extension_host::pack_fs::{pack_dir, packs_root};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 与 `packs/tool-runtime/pack.json` 中的 id 一致。
pub const TOOL_RUNTIME_PACK_ID: &str = "awam.tool-runtime";

pub const CONFIG_FILE_NAME: &str = "config.json";

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:8766";

fn default_base_url() -> String {
    DEFAULT_BASE_URL.to_string()
}

fn default_prefer_attach() -> bool {
    true
}

/// 工具管理运行时 Pack 私有配置（存于已安装 Pack 目录内）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolRuntimeConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_prefer_attach")]
    pub prefer_attach: bool,
    #[serde(default)]
    pub auto_start_sidecar: bool,
    #[serde(default)]
    pub last_sidecar_pid: Option<u32>,
}

impl Default for ToolRuntimeConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            prefer_attach: default_prefer_attach(),
            auto_start_sidecar: false,
            last_sidecar_pid: None,
        }
    }
}

/// `{app_data_dir}/feature-packs/awam.tool-runtime/config.json`
pub fn config_file_path(app_data_dir: &Path) -> PathBuf {
    pack_dir(&packs_root(app_data_dir), TOOL_RUNTIME_PACK_ID).join(CONFIG_FILE_NAME)
}

/// 读取配置；不存在时写入默认值并返回。
pub fn load_from_path(app_data_dir: &Path) -> Result<ToolRuntimeConfig, String> {
    let path = config_file_path(app_data_dir);
    if !path.exists() {
        let document = ToolRuntimeConfig::default();
        save_to_path(app_data_dir, &document)?;
        return Ok(document);
    }

    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read tool-runtime config: {err}"))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse tool-runtime config JSON: {err}"))
}

/// 持久化配置到 Pack 私有目录。
pub fn save_to_path(app_data_dir: &Path, document: &ToolRuntimeConfig) -> Result<(), String> {
    let path = config_file_path(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create tool-runtime config directory: {err}"))?;
    }
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize tool-runtime config: {err}"))?;
    fs::write(&path, text)
        .map_err(|err| format!("Failed to write tool-runtime config: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trip_under_temp_dir() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-tool-runtime-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut cfg = ToolRuntimeConfig::default();
        cfg.base_url = "http://127.0.0.1:9999".to_string();
        save_to_path(&dir, &cfg).unwrap();

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded, cfg);
        assert_eq!(
            config_file_path(&dir),
            dir.join("feature-packs")
                .join(TOOL_RUNTIME_PACK_ID)
                .join(CONFIG_FILE_NAME)
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
