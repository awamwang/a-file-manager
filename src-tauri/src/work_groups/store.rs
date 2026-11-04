// SPDX-License-Identifier: GPL-3.0-or-later

use super::sync::default_document;
use super::types::UserWorkGroups;
use std::fs;
use std::path::{Path, PathBuf};

pub const WORK_GROUPS_FILE_NAME: &str = "work-groups.json";

pub fn work_groups_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(WORK_GROUPS_FILE_NAME)
}

pub fn load_from_path(config_dir: &Path) -> Result<UserWorkGroups, String> {
    let path = work_groups_file_path(config_dir);
    if !path.exists() {
        let document = default_document();
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }
    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read work groups: {err}"))?;
    let mut document: UserWorkGroups = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse work groups JSON: {err}"))?;
    if document.work_groups.is_empty() {
        document = default_document();
        save_to_path(config_dir, &document)?;
    }
    if !document
        .work_groups
        .iter()
        .any(|group| group.id == document.active_work_group_id)
    {
        document.active_work_group_id = document.work_groups[0].id.clone();
        save_to_path(config_dir, &document)?;
    }
    Ok(document)
}

pub fn save_to_path(config_dir: &Path, document: &UserWorkGroups) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create work groups directory: {err}"))?;
    let path = work_groups_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize work groups: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write work groups: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_groups::sync::create_work_group_in_list;

    #[test]
    fn persists_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-wg-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut document = load_from_path(&dir).unwrap();
        let (groups, created) =
            create_work_group_in_list(&document.work_groups, Some("Focus".into()));
        document.work_groups = groups;
        document.active_work_group_id = created.id.clone();
        save_to_path(&dir, &document).unwrap();

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded.active_work_group_id, created.id);
        assert!(loaded.work_groups.iter().any(|g| g.name == "Focus"));

        let _ = fs::remove_dir_all(&dir);
    }
}
