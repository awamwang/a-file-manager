// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin APPDATA index for Projects Root + active Project (ADR 0004 / 0007).
//! Project body lives on disk under Projects Root (C2+); this file is not a projects.json blob.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECTS_INDEX_FILE_NAME: &str = "projects-index.json";
pub const INDEX_SCHEMA_VERSION: u32 = 1;
pub const PROJECTS_ROOT_SETTING_KEY: &str = "core.projectsRoot";
pub const DEFAULT_PROJECTS_DIR_NAME: &str = "projects";

pub const ERR_PROJECT_SLUG_EMPTY: &str = "PROJECT_SLUG_EMPTY";
pub const ERR_PROJECT_SLUG_INVALID: &str = "PROJECT_SLUG_INVALID";
pub const ERR_PROJECT_SLUG_TOO_LONG: &str = "PROJECT_SLUG_TOO_LONG";

/// Max folder-name length in Unicode scalar values (not UTF-8 bytes).
pub const MAX_SLUG_LEN: usize = 64;

const WINDOWS_INVALID_FOLDER_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// True when `name` is a Windows reserved device name (CON/PRN/…), ignoring a trailing extension.
pub fn is_windows_reserved_folder_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

/// True when `c` cannot appear in a Windows file/folder name.
pub fn is_windows_invalid_folder_char(c: char) -> bool {
    c.is_control() || WINDOWS_INVALID_FOLDER_CHARS.contains(&c)
}

/// Pointer to the active Project Workspace (slug + absolute path).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveProjectRef {
    pub slug: String,
    pub path: String,
}

/// APPDATA thin index — Root path + activeProject only (ADR 0007).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsIndex {
    pub schema_version: u32,
    /// Absolute Projects Root path (effective).
    pub projects_root: String,
    pub active_project: Option<ActiveProjectRef>,
}

impl ProjectsIndex {
    pub fn empty(projects_root: impl Into<String>) -> Self {
        Self {
            schema_version: INDEX_SCHEMA_VERSION,
            projects_root: projects_root.into(),
            active_project: None,
        }
    }
}

pub fn projects_index_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(PROJECTS_INDEX_FILE_NAME)
}

/// Default Projects Root: `<config_dir>/projects` (= `%APPDATA%\com.a-file-manager.app\projects\`).
pub fn default_projects_root(config_dir: &Path) -> PathBuf {
    config_dir.join(DEFAULT_PROJECTS_DIR_NAME)
}

/// Resolve effective Root: non-empty setting wins; otherwise default under config dir.
pub fn resolve_projects_root(config_dir: &Path, configured: &str) -> PathBuf {
    let trimmed = configured.trim();
    if trimmed.is_empty() {
        default_projects_root(config_dir)
    } else {
        PathBuf::from(trimmed)
    }
}

pub fn normalize_path_string(path: &Path) -> String {
    crate::paths::to_native_fs_path(&path.to_string_lossy())
}

/// On-disk Workspace folder name (= project slug): display name with Windows path sanitization.
/// Allows Unicode; rejects empty, `.` / `..`, control/invalid path chars, trailing `.`/space,
/// reserved device names, and names longer than [`MAX_SLUG_LEN`] characters.
pub fn validate_slug(slug: &str) -> Result<&str, String> {
    let s = slug.trim();
    if s.is_empty() {
        return Err(ERR_PROJECT_SLUG_EMPTY.to_string());
    }
    if s == "." || s == ".." {
        return Err(ERR_PROJECT_SLUG_INVALID.to_string());
    }
    if s.chars().count() > MAX_SLUG_LEN {
        return Err(ERR_PROJECT_SLUG_TOO_LONG.to_string());
    }
    if s.chars().any(is_windows_invalid_folder_char) {
        return Err(ERR_PROJECT_SLUG_INVALID.to_string());
    }
    if s.ends_with('.') || s.ends_with(' ') {
        return Err(ERR_PROJECT_SLUG_INVALID.to_string());
    }
    if is_windows_reserved_folder_name(s) {
        return Err(ERR_PROJECT_SLUG_INVALID.to_string());
    }
    Ok(s)
}

pub fn workspace_path_for_slug(projects_root: &Path, slug: &str) -> Result<PathBuf, String> {
    let slug = validate_slug(slug)?;
    Ok(projects_root.join(slug))
}

fn active_still_under_root(active: &ActiveProjectRef, root: &Path) -> bool {
    let root_norm = normalize_path_string(root).to_ascii_lowercase();
    let path_norm = active.path.replace('/', "\\").to_ascii_lowercase();
    let prefix = if root_norm.ends_with('\\') {
        root_norm
    } else {
        format!("{root_norm}\\")
    };
    path_norm.starts_with(&prefix)
        && Path::new(&active.path)
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(&active.slug))
}

/// Ensure Projects Root exists on disk; return absolute normalized path string.
pub fn ensure_projects_root_dir(root: &Path) -> Result<String, String> {
    fs::create_dir_all(root)
        .map_err(|err| format!("Failed to create Projects Root: {err}"))?;
    let abs = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|err| format!("Failed to resolve Projects Root: {err}"))?
            .join(root)
    };
    Ok(normalize_path_string(&abs))
}

pub fn load_from_path(config_dir: &Path, configured_root: &str) -> Result<ProjectsIndex, String> {
    let resolved = resolve_projects_root(config_dir, configured_root);
    let root_str = ensure_projects_root_dir(&resolved)?;
    let path = projects_index_file_path(config_dir);

    if !path.exists() {
        let document = ProjectsIndex::empty(root_str);
        save_to_path(config_dir, &document)?;
        return Ok(document);
    }

    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read projects index: {err}"))?;
    let mut document: ProjectsIndex = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse projects index JSON: {err}"))?;
    document.schema_version = INDEX_SCHEMA_VERSION;

    if document.projects_root.replace('/', "\\") != root_str {
        document.projects_root = root_str.clone();
        if let Some(ref active) = document.active_project {
            if !active_still_under_root(active, Path::new(&root_str)) {
                document.active_project = None;
            }
        }
    } else {
        document.projects_root = root_str;
    }

    save_to_path(config_dir, &document)?;
    Ok(document)
}

pub fn save_to_path(config_dir: &Path, document: &ProjectsIndex) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create config directory: {err}"))?;
    let path = projects_index_file_path(config_dir);
    let text = serde_json::to_string_pretty(document)
        .map_err(|err| format!("Failed to serialize projects index: {err}"))?;
    fs::write(&path, text).map_err(|err| format!("Failed to write projects index: {err}"))?;
    Ok(())
}

pub fn set_active_project(
    document: &ProjectsIndex,
    slug: Option<&str>,
) -> Result<ProjectsIndex, String> {
    let mut next = document.clone();
    next.schema_version = INDEX_SCHEMA_VERSION;
    match slug {
        None => {
            next.active_project = None;
        }
        Some(raw) => {
            let root = Path::new(&document.projects_root);
            let workspace = workspace_path_for_slug(root, raw)?;
            let slug = validate_slug(raw)?.to_string();
            next.active_project = Some(ActiveProjectRef {
                slug,
                path: normalize_path_string(&workspace),
            });
        }
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-projects-index-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_root_is_under_config_projects() {
        let config = PathBuf::from(r"C:\Users\x\AppData\Roaming\com.a-file-manager.app");
        assert_eq!(
            default_projects_root(&config),
            config.join("projects")
        );
        assert_eq!(
            resolve_projects_root(&config, ""),
            default_projects_root(&config)
        );
        assert_eq!(
            resolve_projects_root(&config, "   "),
            default_projects_root(&config)
        );
        assert_eq!(
            resolve_projects_root(&config, r"D:\Workspaces"),
            PathBuf::from(r"D:\Workspaces")
        );
    }

    #[test]
    fn load_creates_index_and_root_dir() {
        let dir = temp_dir("load");
        let index = load_from_path(&dir, "").unwrap();
        let expected_root = normalize_path_string(&dir.join("projects"));
        assert_eq!(index.schema_version, INDEX_SCHEMA_VERSION);
        assert_eq!(index.projects_root, expected_root);
        assert!(index.active_project.is_none());
        assert!(dir.join("projects").is_dir());
        assert!(projects_index_file_path(&dir).is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn custom_root_round_trip_and_active() {
        let dir = temp_dir("custom");
        let custom_root = dir.join("my-root");
        let configured = custom_root.to_string_lossy().to_string();
        let index = load_from_path(&dir, &configured).unwrap();
        assert!(custom_root.is_dir());
        assert_eq!(
            index.projects_root,
            normalize_path_string(&custom_root)
        );

        let with_active = set_active_project(&index, Some("demo-app")).unwrap();
        save_to_path(&dir, &with_active).unwrap();
        let loaded = load_from_path(&dir, &configured).unwrap();
        let active = loaded.active_project.as_ref().expect("active");
        assert_eq!(active.slug, "demo-app");
        assert_eq!(
            active.path,
            normalize_path_string(&custom_root.join("demo-app"))
        );

        let cleared = set_active_project(&loaded, None).unwrap();
        assert!(cleared.active_project.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn root_change_clears_active_outside_new_root() {
        let dir = temp_dir("root-change");
        let root_a = dir.join("a");
        let root_b = dir.join("b");
        fs::create_dir_all(&root_a).unwrap();
        fs::create_dir_all(&root_b).unwrap();

        let mut index = load_from_path(&dir, &root_a.to_string_lossy()).unwrap();
        index = set_active_project(&index, Some("alpha")).unwrap();
        save_to_path(&dir, &index).unwrap();

        let switched = load_from_path(&dir, &root_b.to_string_lossy()).unwrap();
        assert_eq!(
            switched.projects_root,
            normalize_path_string(&root_b)
        );
        assert!(switched.active_project.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_invalid_slugs() {
        assert_eq!(
            validate_slug("").unwrap_err(),
            ERR_PROJECT_SLUG_EMPTY
        );
        assert_eq!(
            validate_slug("../x").unwrap_err(),
            ERR_PROJECT_SLUG_INVALID
        );
        assert_eq!(
            validate_slug("has:colon").unwrap_err(),
            ERR_PROJECT_SLUG_INVALID
        );
        assert_eq!(
            validate_slug("CON").unwrap_err(),
            ERR_PROJECT_SLUG_INVALID
        );
        assert_eq!(
            validate_slug("foo.").unwrap_err(),
            ERR_PROJECT_SLUG_INVALID
        );
        assert_eq!(
            validate_slug(&"a".repeat(MAX_SLUG_LEN + 1)).unwrap_err(),
            ERR_PROJECT_SLUG_TOO_LONG
        );
        assert_eq!(validate_slug("ok_Name-1").unwrap(), "ok_Name-1");
        assert_eq!(validate_slug("has space").unwrap(), "has space");
        assert_eq!(validate_slug("-leading").unwrap(), "-leading");
        assert_eq!(validate_slug("视频制作").unwrap(), "视频制作");
    }

    #[test]
    fn index_is_not_projects_json_blob() {
        // Guardrail: thin index file name stays distinct from legacy projects.json.
        assert_ne!(PROJECTS_INDEX_FILE_NAME, super::super::store::PROJECTS_FILE_NAME);
        let dir = temp_dir("thin");
        let index = load_from_path(&dir, "").unwrap();
        let text = fs::read_to_string(projects_index_file_path(&dir)).unwrap();
        assert!(!text.contains("\"projects\""));
        assert!(text.contains("projectsRoot"));
        assert!(text.contains("activeProject"));
        assert_eq!(index.active_project, None);
        let _ = fs::remove_dir_all(&dir);
    }
}
