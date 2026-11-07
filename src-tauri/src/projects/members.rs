// SPDX-License-Identifier: GPL-3.0-or-later
//! Workspace `members/` directory junctions (AIA-210 / C4 · ADR 0007).
//!
//! Junctions point at real folders. Navigator / tabs must open the **target**
//! (original) path — never the junction path under `members/`.

use super::index::normalize_path_string;
use super::ops::basename_label;
use super::workspace::{load_workspace, WORKSPACE_MEMBERS_DIR};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const ERR_MEMBER_TARGET_INVALID: &str = "PROJECT_MEMBER_TARGET_INVALID";
pub const ERR_MEMBER_EXISTS: &str = "PROJECT_MEMBER_EXISTS";
pub const ERR_MEMBER_NOT_FOUND: &str = "PROJECT_MEMBER_NOT_FOUND";
pub const ERR_MEMBER_NAME_INVALID: &str = "PROJECT_MEMBER_NAME_INVALID";

/// One Workspace member junction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMember {
    /// Link name under `members/`.
    pub name: String,
    /// Absolute junction path (`…/<slug>/members/<name>`).
    pub junction_path: String,
    /// Absolute real folder the junction points at.
    pub target_path: String,
    /// Path Navigator / tabs must open (= `target_path`).
    pub open_path: String,
}

fn members_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join(WORKSPACE_MEMBERS_DIR)
}

fn strip_extended_path(path: &str) -> String {
    let trimmed = path.trim();
    let without = trimmed
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| trimmed.strip_prefix(r"\\?\").map(|s| s.to_string()))
        .unwrap_or_else(|| trimmed.to_string());
    normalize_path_string(Path::new(&without))
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|err| format!("Failed to resolve path: {err}"))
    }
}

/// Resolve a junction / directory symlink to its target; otherwise return the path itself.
pub fn resolve_open_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    let path = PathBuf::from(trimmed);
    let abs = absolute_path(&path)?;
    Ok(strip_extended_path(&resolve_reparse_to_target(&abs)?
        .to_string_lossy()))
}

/// If `path` is a reparse point (junction / symlink), return its target (absolute when possible).
pub fn resolve_reparse_to_target(path: &Path) -> Result<PathBuf, String> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => return Ok(path.to_path_buf()),
    };
    if !meta.file_type().is_symlink() {
        return Ok(path.to_path_buf());
    }
    let target = fs::read_link(path)
        .map_err(|err| format!("Failed to read reparse target: {err}"))?;
    let resolved = if target.is_absolute() {
        target
    } else {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target)
    };
    absolute_path(&resolved)
}

fn validate_member_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
    {
        return Err(ERR_MEMBER_NAME_INVALID.to_string());
    }
    Ok(name)
}

fn unique_member_name(members_root: &Path, preferred: &str) -> Result<String, String> {
    let base = validate_member_name(preferred)?.to_string();
    for n in 0..200u32 {
        let candidate = if n == 0 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        if !members_root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(ERR_MEMBER_EXISTS.to_string())
}

/// Create a Windows directory junction under `members/` pointing at `target_path`.
pub fn add_member_junction(
    workspace_dir: &Path,
    target_path: &str,
    link_name: Option<&str>,
) -> Result<WorkspaceMember, String> {
    let trimmed = target_path.trim();
    if trimmed.is_empty() {
        return Err(ERR_MEMBER_TARGET_INVALID.to_string());
    }
    let target = absolute_path(Path::new(trimmed))?;
    if !target.is_dir() {
        return Err(ERR_MEMBER_TARGET_INVALID.to_string());
    }
    // Reject nesting a junction onto something already inside this workspace members tree.
    let members_root = members_dir(workspace_dir);
    fs::create_dir_all(&members_root)
        .map_err(|err| format!("Failed to ensure members/: {err}"))?;

    let preferred = link_name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| basename_label(&normalize_path_string(&target)));
    if preferred.is_empty() {
        return Err(ERR_MEMBER_NAME_INVALID.to_string());
    }

    let name = match link_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(exact) => {
            let name = validate_member_name(exact)?.to_string();
            if members_root.join(&name).exists() {
                return Err(ERR_MEMBER_EXISTS.to_string());
            }
            name
        }
        None => unique_member_name(&members_root, &preferred)?,
    };

    let link = members_root.join(&name);
    create_directory_junction(&link, &target)?;

    let target_norm = strip_extended_path(&normalize_path_string(&target));
    Ok(WorkspaceMember {
        name,
        junction_path: normalize_path_string(&link),
        target_path: target_norm.clone(),
        open_path: target_norm,
    })
}

fn create_directory_junction(link: &Path, target: &Path) -> Result<(), String> {
    if link.exists() {
        return Err(ERR_MEMBER_EXISTS.to_string());
    }
    let link_s = normalize_path_string(link);
    let target_s = strip_extended_path(
        &fs::canonicalize(target)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| normalize_path_string(target)),
    );

    // Directory junction (mklink /J) — no admin required; remove_dir drops link only.
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J", &link_s, &target_s])
        .output()
        .map_err(|err| format!("Failed to run mklink: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Failed to create junction: {} {}",
            stdout.trim(),
            stderr.trim()
        ));
    }
    Ok(())
}

/// Remove a member junction only — never deletes the real target folder.
pub fn remove_member_junction(workspace_dir: &Path, name: &str) -> Result<(), String> {
    let name = validate_member_name(name)?;
    let link = members_dir(workspace_dir).join(name);
    if !link.exists() {
        return Err(ERR_MEMBER_NOT_FOUND.to_string());
    }
    // Junctions / symlink dirs: remove_dir unlinks without touching the target.
    fs::remove_dir(&link).map_err(|err| format!("Failed to remove member junction: {err}"))?;
    Ok(())
}

/// List junctions under `members/` with open paths = real targets.
pub fn list_member_junctions(workspace_dir: &Path) -> Result<Vec<WorkspaceMember>, String> {
    let members_root = members_dir(workspace_dir);
    if !members_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(&members_root)
        .map_err(|err| format!("Failed to read members/: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed to read members/ entry: {err}"))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if validate_member_name(name).is_err() {
            continue;
        }
        let target = match resolve_reparse_to_target(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        // Skip non-reparse plain dirs (should not appear in healthy workspaces).
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.file_type().is_symlink() {
            continue;
        }
        let target_norm = strip_extended_path(&normalize_path_string(&target));
        out.push(WorkspaceMember {
            name: name.to_string(),
            junction_path: normalize_path_string(&path),
            target_path: target_norm.clone(),
            open_path: target_norm,
        });
    }
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    Ok(out)
}

/// Add a member junction for workspace slug under Projects Root.
pub fn add_member_for_slug(
    projects_root: &Path,
    slug: &str,
    target_path: &str,
    link_name: Option<&str>,
) -> Result<WorkspaceMember, String> {
    let item = load_workspace(projects_root, slug)?;
    add_member_junction(Path::new(&item.path), target_path, link_name)
}

pub fn list_members_for_slug(
    projects_root: &Path,
    slug: &str,
) -> Result<Vec<WorkspaceMember>, String> {
    let item = load_workspace(projects_root, slug)?;
    list_member_junctions(Path::new(&item.path))
}

pub fn remove_member_for_slug(
    projects_root: &Path,
    slug: &str,
    name: &str,
) -> Result<(), String> {
    let item = load_workspace(projects_root, slug)?;
    remove_member_junction(Path::new(&item.path), name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::workspace::create_workspace_at;

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-members-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn add_list_resolve_and_remove_preserves_target() {
        let root = temp_root("round");
        let ws = create_workspace_at(&root, "App", None, Some("app"), None).unwrap();
        let target = root.join("real-folder");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("marker.txt"), "keep").unwrap();

        let member =
            add_member_junction(Path::new(&ws.path), &target.to_string_lossy(), None).unwrap();
        assert_eq!(member.name, "real-folder");
        assert!(Path::new(&member.junction_path).exists());
        assert_eq!(member.open_path, member.target_path);
        assert!(
            member
                .open_path
                .eq_ignore_ascii_case(&normalize_path_string(&target))
                || Path::new(&member.open_path)
                    .canonicalize()
                    .ok()
                    .map(|p| strip_extended_path(&p.to_string_lossy())
                        .eq_ignore_ascii_case(&normalize_path_string(&target)))
                    .unwrap_or(false)
        );

        let resolved = resolve_open_path(&member.junction_path).unwrap();
        assert!(
            resolved.eq_ignore_ascii_case(&member.target_path)
                || Path::new(&resolved)
                    .canonicalize()
                    .ok()
                    .zip(Path::new(&member.target_path).canonicalize().ok())
                    .is_some_and(|(a, b)| a == b)
        );

        let listed = list_member_junctions(Path::new(&ws.path)).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].open_path, listed[0].target_path);

        remove_member_junction(Path::new(&ws.path), &member.name).unwrap();
        assert!(!Path::new(&member.junction_path).exists());
        assert!(target.join("marker.txt").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_plain_path_unchanged() {
        let root = temp_root("plain");
        let dir = root.join("folder");
        fs::create_dir_all(&dir).unwrap();
        let resolved = resolve_open_path(&dir.to_string_lossy()).unwrap();
        assert!(
            Path::new(&resolved)
                .canonicalize()
                .unwrap()
                .eq(&dir.canonicalize().unwrap())
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn exact_name_conflict_errors() {
        let root = temp_root("conflict");
        let ws = create_workspace_at(&root, "App", None, Some("app"), None).unwrap();
        let a = root.join("a");
        let b = root.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        add_member_junction(Path::new(&ws.path), &a.to_string_lossy(), Some("shared")).unwrap();
        let err =
            add_member_junction(Path::new(&ws.path), &b.to_string_lossy(), Some("shared"))
                .unwrap_err();
        assert_eq!(err, ERR_MEMBER_EXISTS);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_missing_target() {
        let root = temp_root("missing-target");
        let ws = create_workspace_at(&root, "App", None, Some("app"), None).unwrap();
        let err = add_member_junction(
            Path::new(&ws.path),
            &root.join("nope").to_string_lossy(),
            None,
        )
        .unwrap_err();
        assert_eq!(err, ERR_MEMBER_TARGET_INVALID);
        let _ = fs::remove_dir_all(&root);
    }
}
