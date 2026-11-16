// SPDX-License-Identifier: GPL-3.0-or-later
//! 快捷键自定义覆盖落盘：`{app_config_dir}/shortcut-overrides.json`。
//! 只存和弦数组；动作表在前端 registry.ts，未知 action id 由前端过滤。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub const SHORTCUT_OVERRIDES_FILE_NAME: &str = "shortcut-overrides.json";

/// 与前端 `MAX_CHORDS_PER_ACTION` 一致：防止坏档灌爆 dispatch map。
pub const MAX_CHORDS_PER_ACTION: usize = 8;
pub const MAX_ACTIONS: usize = 512;
const MAX_KEY_LEN: usize = 24;
const MAX_ACTION_ID_LEN: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutChord {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctrl: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<bool>,
}

pub type ShortcutOverrideMap = BTreeMap<String, Vec<ShortcutChord>>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutOverridesDocument {
    pub version: u32,
    pub overrides: ShortcutOverrideMap,
}

impl Default for ShortcutOverridesDocument {
    fn default() -> Self {
        Self {
            version: 1,
            overrides: ShortcutOverrideMap::new(),
        }
    }
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

pub fn overrides_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SHORTCUT_OVERRIDES_FILE_NAME)
}

pub fn default_document() -> ShortcutOverridesDocument {
    ShortcutOverridesDocument::default()
}

/// 轻量净化：非法键丢弃、每动作和弦去重封顶、动作数封顶；
/// 空数组合法（显式解绑），予以保留。
pub fn sanitize_overrides(raw: &ShortcutOverrideMap) -> ShortcutOverrideMap {
    let mut clean = ShortcutOverrideMap::new();
    for (action_id, chords) in raw {
        if action_id.is_empty() || action_id.len() > MAX_ACTION_ID_LEN {
            continue;
        }
        let mut seen: BTreeSet<ShortcutChord> = BTreeSet::new();
        let mut next: Vec<ShortcutChord> = Vec::new();
        for chord in chords {
            if chord.key.is_empty() || chord.key.len() > MAX_KEY_LEN {
                continue;
            }
            let normalized = ShortcutChord {
                key: chord.key.clone(),
                ctrl: chord.ctrl.filter(|&flag| flag),
                shift: chord.shift.filter(|&flag| flag),
                alt: chord.alt.filter(|&flag| flag),
                meta: chord.meta.filter(|&flag| flag),
            };
            if seen.insert(normalized.clone()) {
                next.push(normalized);
            }
            if next.len() >= MAX_CHORDS_PER_ACTION {
                break;
            }
        }
        clean.insert(action_id.clone(), next);
        if clean.len() >= MAX_ACTIONS {
            break;
        }
    }
    clean
}

pub fn load_from_path(config_dir: &Path) -> Result<ShortcutOverridesDocument, String> {
    let path = overrides_file_path(config_dir);
    if !path.exists() {
        let document = default_document();
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read shortcut overrides: {err}"))?;
    let document: ShortcutOverridesDocument = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse shortcut overrides JSON: {err}"))?;
    Ok(ShortcutOverridesDocument {
        version: 1,
        overrides: sanitize_overrides(&document.overrides),
    })
}

pub fn save_to_path(config_dir: &Path, document: &ShortcutOverridesDocument) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create shortcut overrides directory: {err}"))?;
    let path = overrides_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize shortcut overrides: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write shortcut overrides: {err}"))?;
    Ok(())
}

#[tauri::command]
pub fn shortcuts_overrides_get(app: AppHandle) -> Result<ShortcutOverridesDocument, String> {
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

#[tauri::command]
pub fn shortcuts_overrides_set(
    app: AppHandle,
    overrides: ShortcutOverrideMap,
) -> Result<ShortcutOverridesDocument, String> {
    let dir = config_dir(&app)?;
    let document = ShortcutOverridesDocument {
        version: 1,
        overrides: sanitize_overrides(&overrides),
    };
    save_to_path(&dir, &document)?;
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_dedupes_caps_and_keeps_empty_unbind() {
        let mut raw = ShortcutOverrideMap::new();
        raw.insert(
            "navigator.back".to_string(),
            vec![
                ShortcutChord {
                    key: "f7".to_string(),
                    ctrl: None,
                    shift: None,
                    alt: None,
                    meta: None,
                },
                ShortcutChord {
                    key: "f7".to_string(),
                    ctrl: Some(false),
                    shift: None,
                    alt: None,
                    meta: None,
                },
                ShortcutChord {
                    key: "k".to_string(),
                    ctrl: Some(true),
                    shift: None,
                    alt: None,
                    meta: None,
                },
            ],
        );
        // 空数组合法：显式解绑。
        raw.insert("navigator.previewBridge".to_string(), Vec::new());
        raw.insert("".to_string(), Vec::new());
        raw.insert("x".repeat(129), Vec::new());

        let clean = sanitize_overrides(&raw);
        assert_eq!(clean.len(), 2);
        assert_eq!(clean.get("navigator.back").map(Vec::len), Some(2));
        assert_eq!(clean.get("navigator.previewBridge").map(Vec::len), Some(0));
    }

    #[test]
    fn persists_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-shortcut-overrides-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let first = load_from_path(&dir).unwrap();
        assert!(first.overrides.is_empty());

        let mut raw = ShortcutOverrideMap::new();
        raw.insert(
            "navigator.back".to_string(),
            vec![ShortcutChord {
                key: "f7".to_string(),
                ctrl: None,
                shift: None,
                alt: None,
                meta: None,
            }],
        );
        let saved = ShortcutOverridesDocument {
            version: 1,
            overrides: sanitize_overrides(&raw),
        };
        save_to_path(&dir, &saved).unwrap();

        // wire 形状：camelCase 修饰键省略 + 顶层 overrides。
        let raw_text = fs::read_to_string(overrides_file_path(&dir)).unwrap();
        assert!(raw_text.contains("\"overrides\""), "missing overrides: {raw_text}");
        assert!(
            !raw_text.contains("\"ctrl\""),
            "falsy modifiers must be omitted: {raw_text}"
        );

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded, saved);

        let _ = fs::remove_dir_all(&dir);
    }
}
