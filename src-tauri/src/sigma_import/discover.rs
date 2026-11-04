// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::{Path, PathBuf};

/// Default Sigma Tauri identifier config folder (Roaming).
pub const SIGMA_APP_ID: &str = "com.sigma-file-manager.app";
pub const AFM_APP_ID: &str = "com.a-file-manager.app";
pub const USER_DATA_DIR: &str = "user-data";

pub const WORK_GROUPS_FILE: &str = "user-work-groups.json";
pub const PROJECTS_FILE: &str = "user-projects.json";
pub const WORKSPACES_FILE: &str = "user-workspaces.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigmaSourcePaths {
    pub root: PathBuf,
    pub user_data_dir: PathBuf,
    pub work_groups: PathBuf,
    pub projects: PathBuf,
    pub workspaces: PathBuf,
}

pub fn default_afm_config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join(AFM_APP_ID))
}

pub fn default_sigma_roaming_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join(SIGMA_APP_ID))
}

/// Candidate roots searched for Sigma persisted user-data.
pub fn candidate_sigma_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(roaming) = default_sigma_roaming_dir() {
        roots.push(roaming);
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join(SIGMA_APP_ID));
    }
    // Repo checkouts sometimes hold fixture copies under docs / test data — not primary.
    roots.push(PathBuf::from(r"G:\Projects\Tools\sigma-file-manager"));
    roots.push(PathBuf::from(r"G:\Projects\Tools\a-file-manager-sigma"));
    roots
}

pub fn discover_sigma_user_data() -> Result<SigmaSourcePaths, String> {
    let mut searched = Vec::new();
    for root in candidate_sigma_roots() {
        searched.push(root.display().to_string());
        if let Some(found) = try_sigma_root(&root) {
            return Ok(found);
        }
    }
    Err(format!(
        "Sigma user-data not found. Searched:\n  - {}",
        searched.join("\n  - ")
    ))
}

pub fn try_sigma_root(root: &Path) -> Option<SigmaSourcePaths> {
    let user_data = if root.join(USER_DATA_DIR).join(WORK_GROUPS_FILE).is_file() {
        root.join(USER_DATA_DIR)
    } else if root
        .join(SIGMA_APP_ID)
        .join(USER_DATA_DIR)
        .join(WORK_GROUPS_FILE)
        .is_file()
    {
        // unlikely nested
        root.join(SIGMA_APP_ID).join(USER_DATA_DIR)
    } else if root.join(WORK_GROUPS_FILE).is_file() {
        root.to_path_buf()
    } else {
        return None;
    };

    let work_groups = user_data.join(WORK_GROUPS_FILE);
    let projects = user_data.join(PROJECTS_FILE);
    let workspaces = user_data.join(WORKSPACES_FILE);
    if !work_groups.is_file() {
        return None;
    }

    Some(SigmaSourcePaths {
        root: root.to_path_buf(),
        user_data_dir: user_data,
        work_groups,
        projects,
        workspaces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discovers_user_data_layout() {
        let dir = std::env::temp_dir().join(format!(
            "afm-sigma-discover-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let user_data = dir.join(USER_DATA_DIR);
        fs::create_dir_all(&user_data).unwrap();
        fs::write(user_data.join(WORK_GROUPS_FILE), r#"{"workGroups":[],"activeWorkGroupId":"default-work-group"}"#)
            .unwrap();
        fs::write(
            user_data.join(PROJECTS_FILE),
            r#"{"schemaVersion":2,"projects":[],"activeProjectId":null}"#,
        )
        .unwrap();
        fs::write(
            user_data.join(WORKSPACES_FILE),
            r#"{"__schemaVersion":1,"workspaces":[]}"#,
        )
        .unwrap();

        let found = try_sigma_root(&dir).expect("should find");
        assert_eq!(found.work_groups, user_data.join(WORK_GROUPS_FILE));
        let _ = fs::remove_dir_all(&dir);
    }
}
