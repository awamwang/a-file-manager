// SPDX-License-Identifier: GPL-3.0-or-later
//! App-local favorites (AIA-57). Explorer shell verb registration is deferred.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

pub const FAVORITES_FILE_NAME: &str = "favorites.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub path: String,
    pub added_at: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesDocument {
    pub favorites: Vec<FavoriteItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesToggleResult {
    pub document: FavoritesDocument,
    pub added: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesOsHookStatus {
    /// Explorer COM / shell-verb registration for "Add to AFM favorites".
    pub registered: bool,
    pub note: String,
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

fn favorites_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(FAVORITES_FILE_NAME)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn normalize_path(path: &str) -> String {
    path.trim().replace('/', "\\")
}

pub fn load_from_path(config_dir: &Path) -> Result<FavoritesDocument, String> {
    let path = favorites_file_path(config_dir);
    if !path.exists() {
        let document = FavoritesDocument::default();
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }
    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read favorites: {err}"))?;
    let document: FavoritesDocument = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse favorites JSON: {err}"))?;
    Ok(document)
}

pub fn save_to_path(config_dir: &Path, document: &FavoritesDocument) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create favorites directory: {err}"))?;
    let path = favorites_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize favorites: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write favorites: {err}"))?;
    Ok(())
}

pub fn is_favorite(document: &FavoritesDocument, path: &str) -> bool {
    let needle = normalize_path(path);
    document
        .favorites
        .iter()
        .any(|item| normalize_path(&item.path).eq_ignore_ascii_case(&needle))
}

pub fn toggle_favorite(
    document: &mut FavoritesDocument,
    path: &str,
    is_dir: bool,
) -> Result<bool, String> {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        return Err("Path is empty".to_string());
    }
    if let Some(index) = document
        .favorites
        .iter()
        .position(|item| normalize_path(&item.path).eq_ignore_ascii_case(&normalized))
    {
        document.favorites.remove(index);
        return Ok(false);
    }
    document.favorites.push(FavoriteItem {
        path: normalized,
        added_at: now_ms(),
        is_dir,
    });
    Ok(true)
}

#[tauri::command]
pub fn favorites_get(app: AppHandle) -> Result<FavoritesDocument, String> {
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

#[tauri::command]
pub fn favorites_toggle(
    app: AppHandle,
    path: String,
    is_dir: bool,
) -> Result<FavoritesToggleResult, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    let added = toggle_favorite(&mut document, &path, is_dir)?;
    save_to_path(&dir, &document)?;
    Ok(FavoritesToggleResult {
        path: normalize_path(&path),
        added,
        document,
    })
}

#[tauri::command]
pub fn favorites_contains(app: AppHandle, path: String) -> Result<bool, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    Ok(is_favorite(&document, &path))
}

/// Day-1: report that Explorer shell-verb registration is not installed.
/// AFM favorites remain app-local; use System menu (AIA-55) for Explorer Quick Access.
#[tauri::command]
pub fn favorites_os_hook_status() -> FavoritesOsHookStatus {
    FavoritesOsHookStatus {
        registered: false,
        note: "Explorer shell verb for AFM favorites is deferred. Use in-app Add to favorites; System menu covers OS Quick Access.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_adds_and_removes() {
        let mut doc = FavoritesDocument::default();
        assert!(toggle_favorite(&mut doc, r"C:\Docs", true).unwrap());
        assert!(is_favorite(&doc, r"C:/Docs"));
        assert!(!toggle_favorite(&mut doc, r"C:\Docs", true).unwrap());
        assert!(!is_favorite(&doc, r"C:\Docs"));
    }

    #[test]
    fn persists_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-fav-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut document = load_from_path(&dir).unwrap();
        toggle_favorite(&mut document, r"D:\Work", true).unwrap();
        save_to_path(&dir, &document).unwrap();

        let loaded = load_from_path(&dir).unwrap();
        assert!(is_favorite(&loaded, r"D:\Work"));
        assert_eq!(favorites_os_hook_status().registered, false);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_path_errors() {
        let mut doc = FavoritesDocument::default();
        assert!(toggle_favorite(&mut doc, "  ", false).is_err());
    }
}
