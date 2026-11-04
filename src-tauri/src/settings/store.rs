// SPDX-License-Identifier: GPL-3.0-or-later

use super::defaults::default_document;
use super::migrate::{apply_patch, migrate_to_latest};
use super::types::SettingsDocument;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const SETTINGS_FILE_NAME: &str = "settings.json";

pub fn settings_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SETTINGS_FILE_NAME)
}

pub fn load_from_path(config_dir: &Path) -> Result<SettingsDocument, String> {
    let path = settings_file_path(config_dir);
    if !path.exists() {
        let document = default_document();
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }

    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read settings: {err}"))?;
    let raw: Value = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse settings JSON: {err}"))?;
    let migrated = migrate_to_latest(&raw)?;
    if migrated.migrated {
        save_to_path(config_dir, &migrated.document)?;
    } else {
        // Persist sanitized repairs (unknown keys dropped, bad enums fixed).
        save_to_path(config_dir, &migrated.document)?;
    }
    Ok(migrated.document)
}

pub fn save_to_path(config_dir: &Path, document: &SettingsDocument) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create settings directory: {err}"))?;
    let path = settings_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize settings: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write settings: {err}"))?;
    Ok(())
}

pub fn set_values_at_path(
    config_dir: &Path,
    patch: BTreeMap<String, Value>,
) -> Result<SettingsDocument, String> {
    let current = load_from_path(config_dir)?;
    let next = apply_patch(current, patch)?;
    save_to_path(config_dir, &next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn persists_round_trip_under_temp_dir() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-settings-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut patch = BTreeMap::new();
        patch.insert("core.theme".to_string(), json!("dark"));
        let saved = set_values_at_path(&dir, patch).unwrap();
        assert_eq!(saved.values.get("core.theme"), Some(&json!("dark")));

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded.values.get("core.theme"), Some(&json!("dark")));
        assert_eq!(
            loaded.values.get("core.listDensity"),
            Some(&json!("standard"))
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
