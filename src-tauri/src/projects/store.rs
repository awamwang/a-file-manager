// SPDX-License-Identifier: GPL-3.0-or-later

use super::migrate::{empty_document, migrate_value};
use super::types::UserProjects;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECTS_FILE_NAME: &str = "projects.json";

pub fn projects_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(PROJECTS_FILE_NAME)
}

#[allow(dead_code)] // retained for migrate/tests; Workspace path no longer loads via IPC
pub fn load_from_path(config_dir: &Path) -> Result<UserProjects, String> {
    let path = projects_file_path(config_dir);
    if !path.exists() {
        let document = empty_document();
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }
    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read projects: {err}"))?;
    let raw: serde_json::Value = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse projects JSON: {err}"))?;
    let document = migrate_value(raw)?;
    save_to_path(config_dir, &document)?;
    Ok(document)
}

#[allow(dead_code)] // retained for migrate/tests; Workspace path no longer saves via IPC
pub fn save_to_path(config_dir: &Path, document: &UserProjects) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create projects directory: {err}"))?;
    let path = projects_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize projects: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write projects: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::ops::{add_task, create_project};

    #[test]
    fn persists_round_trip_with_tasks() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-projects-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut document = load_from_path(&dir).unwrap();
        let (next, created) =
            create_project(&document, Some("Core".into()), Some("desc".into()), None);
        document = next;
        let (next, _task) = add_task(&document, &created.id, "First".into(), None).unwrap();
        document = next;
        save_to_path(&dir, &document).unwrap();

        let loaded = load_from_path(&dir).unwrap();
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "Core");
        assert_eq!(loaded.projects[0].tasks.len(), 1);
        assert_eq!(loaded.projects[0].tasks[0].status_id, "backlog");

        let _ = fs::remove_dir_all(&dir);
    }
}
