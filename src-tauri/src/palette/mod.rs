// SPDX-License-Identifier: GPL-3.0-or-later
//! 统一搜索面板 Core 配置：`<appConfigDir>/command-palette.json`。
//! 文件缺失 / JSON 损坏一律回落空配置，不阻断面板（见 docs/command-palette-config.md）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub const PALETTE_CONFIG_FILE_NAME: &str = "command-palette.json";
pub const PALETTE_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    PALETTE_SCHEMA_VERSION
}

fn default_enter() -> String {
    "navigate".to_string()
}

/// 单个工具的 Enter 动作覆盖（缺省 `navigate`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteToolAction {
    /// `navigate` | `callCapability`；非法值由前端回落 `navigate`。
    #[serde(default = "default_enter")]
    pub enter: String,
    #[serde(default)]
    pub capability_id: Option<String>,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// `command-palette.json` 根文档。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaletteConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub tool_actions: BTreeMap<String, PaletteToolAction>,
}

impl Default for PaletteConfig {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            tool_actions: BTreeMap::new(),
        }
    }
}

fn app_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

/// `<appConfigDir>/command-palette.json`。
pub fn palette_config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(PALETTE_CONFIG_FILE_NAME)
}

/// 读取配置；文件缺失 / JSON 损坏回落空配置（不报错、不阻断面板）。
pub fn load_from_path(config_dir: &Path) -> PaletteConfig {
    let path = palette_config_path(config_dir);
    if !path.exists() {
        return PaletteConfig::default();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return PaletteConfig::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// 读取统一搜索面板配置（v1 仅 tools 源覆盖 Enter 动作时消费）。
#[tauri::command]
pub async fn palette_get_config(app: AppHandle) -> Result<PaletteConfig, String> {
    let dir = app_config_dir(&app)?;
    Ok(tauri::async_runtime::spawn_blocking(move || load_from_path(&dir))
        .await
        .map_err(|err| format!("palette_get_config worker failed: {err}"))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-palette-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_file_falls_back_to_default() {
        let dir = temp_dir("missing");
        let config = load_from_path(&dir);
        assert_eq!(config, PaletteConfig::default());
        assert_eq!(config.schema_version, PALETTE_SCHEMA_VERSION);
        assert!(config.tool_actions.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_json_falls_back_to_default() {
        let dir = temp_dir("corrupt");
        fs::write(palette_config_path(&dir), "{ not json").unwrap();
        assert_eq!(load_from_path(&dir), PaletteConfig::default());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn valid_document_parses_with_defaults() {
        let dir = temp_dir("valid");
        fs::write(
            palette_config_path(&dir),
            r#"{
  "schemaVersion": 1,
  "toolActions": {
    "some-tool": { "enter": "navigate" },
    "another-tool": {
      "enter": "callCapability",
      "capabilityId": "run",
      "arguments": { "flag": true }
    }
  }
}"#,
        )
        .unwrap();
        let config = load_from_path(&dir);
        assert_eq!(config.schema_version, 1);
        assert_eq!(config.tool_actions.len(), 2);

        let simple = &config.tool_actions["some-tool"];
        assert_eq!(simple.enter, "navigate");
        assert_eq!(simple.capability_id, None);
        assert_eq!(simple.arguments, None);

        let call = &config.tool_actions["another-tool"];
        assert_eq!(call.enter, "callCapability");
        assert_eq!(call.capability_id.as_deref(), Some("run"));
        assert_eq!(
            call.arguments,
            Some(serde_json::json!({ "flag": true }))
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_action_uses_field_defaults() {
        let dir = temp_dir("partial");
        fs::write(
            palette_config_path(&dir),
            r#"{ "toolActions": { "some-tool": {} } }"#,
        )
        .unwrap();
        let config = load_from_path(&dir);
        let action = &config.tool_actions["some-tool"];
        assert_eq!(action.enter, "navigate");
        assert_eq!(action.capability_id, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_path_joins_file_name() {
        let dir = temp_dir("path");
        assert_eq!(
            palette_config_path(&dir),
            dir.join(PALETTE_CONFIG_FILE_NAME)
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
