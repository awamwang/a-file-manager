// SPDX-License-Identifier: GPL-3.0-or-later
//! Kernel file operations: copy / move / rename / mkdir / recycle-delete (AIA-62).
//! Recycle Bin is the default delete. Permanent delete is a separate confirmed command.
//! AIA-275: disk-safe move/copy via `safe_io` (staging + verify before source delete).

pub(crate) mod clipboard;
pub(crate) mod new_templates;
pub(crate) mod recycle;
pub(crate) mod safe_io;
pub(crate) mod shortcut;

use crate::navigator::path_guard::assert_os_path;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileOpsResult {
    pub ok: bool,
    pub paths: Vec<String>,
    pub error: Option<String>,
}

fn ok_paths(paths: Vec<String>) -> FileOpsResult {
    FileOpsResult {
        ok: true,
        paths,
        error: None,
    }
}

fn err_msg(message: impl Into<String>) -> FileOpsResult {
    FileOpsResult {
        ok: false,
        paths: Vec::new(),
        error: Some(message.into()),
    }
}

fn display_path(path: &Path) -> String {
    crate::paths::to_native_fs_path(&path.to_string_lossy())
}

fn ensure_existing(path: &str) -> Result<PathBuf, String> {
    assert_os_path(path)?;
    let pb = PathBuf::from(path.trim());
    if !pb.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    Ok(pb)
}

fn ensure_dir(path: &str) -> Result<PathBuf, String> {
    let pb = ensure_existing(path)?;
    if !pb.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }
    Ok(pb)
}

fn validate_file_name(name: &str) -> Result<&str, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name must not be empty".to_string());
    }
    if trimmed.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Name contains invalid characters".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("Name is not allowed".to_string());
    }
    Ok(trimmed)
}

/// Prefer a free destination name: `name`, then `name - Copy`, `name - Copy (2)`, …
pub fn unique_dest_path(parent: &Path, file_name: &str) -> PathBuf {
    let candidate = parent.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.to_string());
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let first = parent.join(format!("{stem} - Copy{ext}"));
    if !first.exists() {
        return first;
    }
    let mut n = 2u32;
    loop {
        let next = parent.join(format!("{stem} - Copy ({n}){ext}"));
        if !next.exists() {
            return next;
        }
        n += 1;
    }
}

fn is_descendant_or_same(ancestor: &Path, maybe_child: &Path) -> bool {
    let a = ancestor.components().collect::<Vec<_>>();
    let c = maybe_child.components().collect::<Vec<_>>();
    c.starts_with(&a)
}

/// Local drives can use the Recycle Bin; UNC / remote targets cannot (Windows).
pub fn path_can_use_recycle_bin(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.starts_with("\\\\") || trimmed.starts_with("//") {
        return false;
    }
    true
}

pub fn delete_to_recycle(path: &Path, path_text: &str) -> Result<(), String> {
    if !path_can_use_recycle_bin(path_text) {
        return Err(
            "Recycle Bin is not available for this location; permanent delete is not enabled"
                .to_string(),
        );
    }
    trash::delete(path).map_err(|err| format!("Failed to move to Recycle Bin: {err}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameConflict {
    KeepBoth,
    Replace,
    Skip,
}

pub fn parse_name_conflict(value: Option<&str>) -> NameConflict {
    match value.unwrap_or("keep") {
        "replace" => NameConflict::Replace,
        "skip" => NameConflict::Skip,
        _ => NameConflict::KeepBoth,
    }
}

#[derive(Debug, Clone)]
struct DestPlan {
    path: PathBuf,
    /// True when an existing same-name entry must be swapped out only after staging verifies.
    replace: bool,
}

/// `Ok(None)` means the existing name was skipped.
/// Does **not** delete an existing destination — replace is deferred to safe_io.
fn place_dest(
    parent: &Path,
    file_name: &str,
    source: &Path,
    conflict: NameConflict,
) -> Result<Option<DestPlan>, String> {
    let exact = parent.join(file_name);
    if exact == source {
        return Ok(Some(DestPlan {
            path: exact,
            replace: false,
        }));
    }
    if !exact.exists() {
        return Ok(Some(DestPlan {
            path: exact,
            replace: false,
        }));
    }
    match conflict {
        NameConflict::KeepBoth => Ok(Some(DestPlan {
            path: unique_dest_path(parent, file_name),
            replace: false,
        })),
        NameConflict::Skip => Ok(None),
        NameConflict::Replace => Ok(Some(DestPlan {
            path: exact,
            replace: true,
        })),
    }
}

pub fn copy_paths(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
) -> Result<Vec<String>, String> {
    let dest = ensure_dir(dest_dir)?;
    let mut out = Vec::new();
    for source in sources {
        let src = ensure_existing(source)?;
        if src.is_dir() && is_descendant_or_same(&src, &dest) {
            return Err(format!(
                "Cannot copy a folder into itself: {}",
                display_path(&src)
            ));
        }
        let name = src
            .file_name()
            .ok_or_else(|| format!("Invalid source path: {source}"))?
            .to_string_lossy()
            .into_owned();
        let Some(plan) = place_dest(&dest, &name, &src, conflict)? else {
            continue;
        };
        safe_io::safe_copy_entry(&src, &plan.path, plan.replace)?;
        out.push(display_path(&plan.path));
    }
    Ok(out)
}

pub fn move_paths(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
) -> Result<Vec<String>, String> {
    let dest = ensure_dir(dest_dir)?;
    let mut out = Vec::new();
    for source in sources {
        let src = ensure_existing(source)?;
        safe_io::refuse_volume_root(&src)?;
        if src.is_dir() && is_descendant_or_same(&src, &dest) {
            return Err(format!(
                "Cannot move a folder into itself: {}",
                display_path(&src)
            ));
        }
        let name = src
            .file_name()
            .ok_or_else(|| format!("Invalid source path: {source}"))?
            .to_string_lossy()
            .into_owned();
        // Same-directory cut is a no-op rename target; skip.
        if src.parent().map(Path::as_os_str) == Some(dest.as_os_str()) {
            out.push(display_path(&src));
            continue;
        }
        let Some(plan) = place_dest(&dest, &name, &src, conflict)? else {
            continue;
        };
        safe_io::safe_move_entry(&src, &plan.path, plan.replace)?;
        out.push(display_path(&plan.path));
    }
    Ok(out)
}

pub fn rename_path(path: &str, new_name: &str) -> Result<String, String> {
    let src = ensure_existing(path)?;
    let name = validate_file_name(new_name)?;
    let parent = src
        .parent()
        .ok_or_else(|| "Cannot rename a root path".to_string())?;
    let dest = parent.join(name);
    if dest.exists() && dest != src {
        return Err(format!("A file named '{name}' already exists"));
    }
    fs::rename(&src, &dest).map_err(|err| format!("Failed to rename: {err}"))?;
    Ok(display_path(&dest))
}

pub fn create_directory(parent: &str, name: &str) -> Result<String, String> {
    let parent_pb = ensure_dir(parent)?;
    let folder_name = validate_file_name(name)?;
    let dest = parent_pb.join(folder_name);
    if dest.exists() {
        return Err(format!("A file named '{folder_name}' already exists"));
    }
    fs::create_dir(&dest).map_err(|err| format!("Failed to create folder: {err}"))?;
    Ok(display_path(&dest))
}

pub fn create_empty_file(parent: &str, name: &str) -> Result<String, String> {
    let parent_pb = ensure_dir(parent)?;
    let file_name = validate_file_name(name)?;
    let dest = parent_pb.join(file_name);
    if dest.exists() {
        return Err(format!("A file named '{file_name}' already exists"));
    }
    fs::File::create(&dest).map_err(|err| format!("Failed to create file: {err}"))?;
    Ok(display_path(&dest))
}

pub fn delete_paths(paths: &[String]) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();
    for path in paths {
        let pb = ensure_existing(path)?;
        safe_io::refuse_volume_root(&pb).map_err(|err| {
            err.replace(
                "destructive op on a volume root",
                "move to Recycle Bin for a volume root",
            )
        })?;
        delete_to_recycle(&pb, path)?;
        deleted.push(display_path(&pb));
    }
    Ok(deleted)
}

pub fn delete_permanent_paths(paths: &[String]) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();
    for path in paths {
        let pb = ensure_existing(path)?;
        safe_io::refuse_volume_root(&pb).map_err(|err| {
            err.replace(
                "destructive op on a volume root",
                "permanently delete a volume root",
            )
        })?;
        if pb.is_dir() {
            fs::remove_dir_all(&pb).map_err(|err| format!("Failed to permanently delete: {err}"))?;
        } else {
            fs::remove_file(&pb).map_err(|err| format!("Failed to permanently delete: {err}"))?;
        }
        deleted.push(display_path(&pb));
    }
    Ok(deleted)
}

#[tauri::command]
pub fn file_ops_name_exists(dest_dir: String, name: String) -> Result<bool, String> {
    let dest = ensure_dir(&dest_dir)?;
    let file_name = validate_file_name(&name)?;
    Ok(dest.join(file_name).exists())
}

/// 大目录复制不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn file_ops_copy(
    sources: Vec<String>,
    dest_dir: String,
    conflict: Option<String>,
) -> FileOpsResult {
    let mode = parse_name_conflict(conflict.as_deref());
    match tauri::async_runtime::spawn_blocking(move || copy_paths(&sources, &dest_dir, mode))
        .await
    {
        Ok(Ok(paths)) => ok_paths(paths),
        Ok(Err(error)) => err_msg(error),
        Err(error) => err_msg(format!("file_ops_copy worker failed: {error}")),
    }
}

/// 大目录移动不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn file_ops_move(
    sources: Vec<String>,
    dest_dir: String,
    conflict: Option<String>,
) -> FileOpsResult {
    let mode = parse_name_conflict(conflict.as_deref());
    match tauri::async_runtime::spawn_blocking(move || move_paths(&sources, &dest_dir, mode))
        .await
    {
        Ok(Ok(paths)) => ok_paths(paths),
        Ok(Err(error)) => err_msg(error),
        Err(error) => err_msg(format!("file_ops_move worker failed: {error}")),
    }
}

#[tauri::command]
pub fn file_ops_rename(path: String, new_name: String) -> FileOpsResult {
    match rename_path(&path, &new_name) {
        Ok(p) => ok_paths(vec![p]),
        Err(error) => err_msg(error),
    }
}

#[tauri::command]
pub fn file_ops_create_dir(parent: String, name: String) -> FileOpsResult {
    match create_directory(&parent, &name) {
        Ok(p) => ok_paths(vec![p]),
        Err(error) => err_msg(error),
    }
}

#[tauri::command]
pub fn file_ops_create_file(parent: String, name: String) -> FileOpsResult {
    match create_empty_file(&parent, &name) {
        Ok(p) => ok_paths(vec![p]),
        Err(error) => err_msg(error),
    }
}

/// 多子文件目录移入回收站不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn file_ops_delete(paths: Vec<String>) -> FileOpsResult {
    match tauri::async_runtime::spawn_blocking(move || delete_paths(&paths)).await {
        Ok(Ok(paths)) => ok_paths(paths),
        Ok(Err(error)) => err_msg(error),
        Err(error) => err_msg(format!("file_ops_delete worker failed: {error}")),
    }
}

/// 永久删除大目录不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn file_ops_delete_permanent(paths: Vec<String>) -> FileOpsResult {
    match tauri::async_runtime::spawn_blocking(move || delete_permanent_paths(&paths)).await {
        Ok(Ok(paths)) => ok_paths(paths),
        Ok(Err(error)) => err_msg(error),
        Err(error) => err_msg(format!("file_ops_delete_permanent worker failed: {error}")),
    }
}

#[tauri::command]
pub fn file_ops_can_recycle(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .filter(|p| !path_can_use_recycle_bin(p))
        .collect()
}

#[tauri::command]
pub fn file_ops_recycle_list() -> Result<Vec<recycle::RecycleBinItem>, String> {
    recycle::list_recycle_bin()
}

#[tauri::command]
pub fn file_ops_restore(paths: Vec<String>) -> FileOpsResult {
    match recycle::restore_original_paths(&paths) {
        Ok(restored) => ok_paths(restored),
        Err(error) => err_msg(error),
    }
}

#[tauri::command]
pub fn file_ops_new_templates() -> Result<Vec<new_templates::NewFileTemplate>, String> {
    new_templates::list_new_file_templates()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_workspace(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-file-ops-{}-{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn unique_dest_avoids_collision() {
        let dir = temp_workspace("unique");
        fs::write(dir.join("a.txt"), b"1").unwrap();
        let first = unique_dest_path(&dir, "a.txt");
        assert_eq!(first.file_name().unwrap(), "a - Copy.txt");
        fs::write(&first, b"2").unwrap();
        let second = unique_dest_path(&dir, "a.txt");
        assert_eq!(second.file_name().unwrap(), "a - Copy (2).txt");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_rename_mkdir_roundtrip() {
        let dir = temp_workspace("round");
        let src = dir.join("src");
        let dest = dir.join("dest");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dest).unwrap();
        let mut f = fs::File::create(src.join("note.txt")).unwrap();
        write!(f, "hello").unwrap();

        let copied = copy_paths(
            &[display_path(&src.join("note.txt"))],
            &display_path(&dest),
            NameConflict::KeepBoth,
        )
        .unwrap();
        assert_eq!(copied.len(), 1);
        assert!(Path::new(&copied[0]).is_file());

        let renamed = rename_path(&copied[0], "renamed.txt").unwrap();
        assert!(Path::new(&renamed).is_file());
        assert!(!Path::new(&copied[0]).exists());

        let folder = create_directory(&display_path(&dest), "New folder").unwrap();
        assert!(Path::new(&folder).is_dir());

        let empty = create_empty_file(&display_path(&dest), "New Text Document.txt").unwrap();
        assert!(Path::new(&empty).is_file());
        assert_eq!(fs::read(&empty).unwrap(), b"");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn move_within_temp() {
        let dir = temp_workspace("move");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("x.bin"), b"xx").unwrap();
        let moved = move_paths(
            &[display_path(&a.join("x.bin"))],
            &display_path(&b),
            NameConflict::KeepBoth,
        )
        .unwrap();
        assert_eq!(moved.len(), 1);
        assert!(Path::new(&moved[0]).is_file());
        assert!(!a.join("x.bin").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_overwrites_and_skip_leaves_existing() {
        let dir = temp_workspace("conflict");
        let src = dir.join("src");
        let dest = dir.join("dest");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(src.join("note.txt"), b"new").unwrap();
        fs::write(dest.join("note.txt"), b"old").unwrap();

        let replaced = copy_paths(
            &[display_path(&src.join("note.txt"))],
            &display_path(&dest),
            NameConflict::Replace,
        )
        .unwrap();
        assert_eq!(replaced.len(), 1);
        assert_eq!(fs::read(dest.join("note.txt")).unwrap(), b"new");

        fs::write(src.join("note.txt"), b"later").unwrap();
        let skipped = copy_paths(
            &[display_path(&src.join("note.txt"))],
            &display_path(&dest),
            NameConflict::Skip,
        )
        .unwrap();
        assert!(skipped.is_empty());
        assert_eq!(fs::read(dest.join("note.txt")).unwrap(), b"new");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_to_recycle_local_file() {
        let dir = temp_workspace("del");
        let file = dir.join("gone.txt");
        fs::write(&file, b"bye").unwrap();
        let path = display_path(&file);
        assert!(path_can_use_recycle_bin(&path));
        delete_paths(&[path]).unwrap();
        assert!(!file.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn permanent_delete_removes_file_and_refuses_root() {
        let dir = temp_workspace("perm");
        let file = dir.join("gone.txt");
        fs::write(&file, b"bye").unwrap();
        let removed = delete_permanent_paths(&[display_path(&file)]).unwrap();
        assert_eq!(removed.len(), 1);
        assert!(!file.exists());
        let err = delete_permanent_paths(&[r"C:\".to_string()]).unwrap_err();
        assert!(err.contains("root"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unc_cannot_recycle() {
        assert!(!path_can_use_recycle_bin(r"\\server\share\file.txt"));
    }

    #[test]
    fn rejects_awam_paths() {
        let err = copy_paths(&["awam://x".into()], r"C:\", NameConflict::KeepBoth).unwrap_err();
        assert!(err.to_ascii_lowercase().contains("awam"));
    }

    #[test]
    fn validate_name_blocks_separators() {
        assert!(validate_file_name("a/b").is_err());
        assert!(validate_file_name("..").is_err());
        assert_eq!(validate_file_name("  ok  ").unwrap(), "ok");
    }
}
