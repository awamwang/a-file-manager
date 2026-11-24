// SPDX-License-Identifier: GPL-3.0-or-later
//! Kernel file operations: copy / move / rename / mkdir / recycle-delete (AIA-62).
//! Recycle Bin is the default delete. Permanent delete is a separate confirmed command.
//! AIA-275: disk-safe move/copy via `safe_io` (staging + verify before source delete).

pub(crate) mod cancel;
pub(crate) mod clipboard;
pub(crate) mod new_templates;
pub(crate) mod progress;
pub(crate) mod recycle;
pub(crate) mod safe_io;
pub(crate) mod shortcut;

use crate::navigator::path_guard::assert_os_path;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileOpsResult {
    pub ok: bool,
    pub paths: Vec<String>,
    pub error: Option<String>,
    /// Localized-error contract: a machine-readable code (e.g. `name_exists`,
    /// `path_missing`) the frontend maps to a locale key `fileOps.error.<code>`.
    /// `None` for successes and for hard refusals that keep the raw `error`
    /// fallback. The English `error` string stays populated for logs + unknown
    /// codes, so this is purely additive / backward-compatible.
    #[serde(default)]
    pub error_code: Option<String>,
    /// Named placeholders (e.g. `name`, `path`) interpolated into the localized
    /// message for `error_code`. Empty/`None` when the code carries no params.
    #[serde(default)]
    pub error_params: Option<std::collections::HashMap<String, String>>,
    /// LC-006 / LC-007 / LC-008: a machine-readable class for a **per-item**
    /// transfer failure so the frontend can offer Retry / Skip / Abort instead of
    /// silently aborting the whole batch. `None` for successes and for hard
    /// refusals (folder-into-itself, volume root, validation) where retry/skip is
    /// meaningless. Populated centrally from the OS error in [`err_msg`].
    #[serde(default)]
    pub error_class: Option<String>,
    /// True when the operation was cancelled by the user (LC-005). Work done so
    /// far has been rolled back where safe; this is distinct from a failure.
    #[serde(default)]
    pub cancelled: bool,
}

/// Recognized per-item transfer error classes (aligned with Explorer's Retry /
/// Skip / Abort dialog). Must stay in sync with `src/state/transferError.ts`.
pub const ERROR_CLASS_IN_USE: &str = "in-use";
pub const ERROR_CLASS_ACCESS_DENIED: &str = "access-denied";
pub const ERROR_CLASS_DISK_FULL: &str = "disk-full";

/// Map a per-item IO failure to the Retry / Skip / Abort class the frontend
/// understands. Rust appends the raw OS error code as `(os error N)` regardless
/// of locale, so the code is the locale-stable signal; the code table is
/// platform-specific (Windows system codes vs POSIX errno). English fragments
/// are a portable fallback for wrapped messages and are consulted even when an
/// embedded code is unrecognized — an unknown code must never short-circuit the
/// fallback (that silently disabled the per-item dialog on macOS, where EBUSY /
/// ENOSPC / EPERM are not Windows codes). `None` means a hard refusal, for which
/// the caller keeps the whole-batch abort behavior.
fn classify_transfer_error(message: &str) -> Option<String> {
    let lower = message.to_ascii_lowercase();
    if let Some(idx) = lower.rfind("(os error ") {
        let after = &lower[idx + "(os error ".len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(code) = digits.parse::<i64>() {
            if let Some(class) = class_from_os_code(code) {
                return Some(class.to_string());
            }
            // Unknown code: fall through to the portable English fragments.
        }
    }
    // Portable fallback: stable English fragments (wrapped messages, or a host
    // whose code table has not been extended yet).
    if lower.contains("being used by another process")
        || lower.contains("used by another process")
        || lower.contains("the process cannot access the file")
        || lower.contains("resource busy")
        || lower.contains("text file busy")
    {
        return Some(ERROR_CLASS_IN_USE.to_string());
    }
    if lower.contains("access is denied")
        || lower.contains("permission denied")
        || lower.contains("operation not permitted")
    {
        return Some(ERROR_CLASS_ACCESS_DENIED.to_string());
    }
    if (lower.contains("not enough") && lower.contains("space"))
        || lower.contains("no space left")
        || lower.contains("disk quota exceeded")
        || lower.contains("disk is full")
        || lower.contains("disk full")
    {
        return Some(ERROR_CLASS_DISK_FULL.to_string());
    }
    None
}

/// Locale-stable OS code → error class. Windows uses system error codes;
/// macOS / Linux use POSIX errno (both embedded verbatim by `io::Error`).
fn class_from_os_code(code: i64) -> Option<&'static str> {
    #[cfg(windows)]
    {
        match code {
            5 | 13 => Some(ERROR_CLASS_ACCESS_DENIED), // ERROR_ACCESS_DENIED / ERROR_INVALID_ACCESS
            32 | 33 | 36 => Some(ERROR_CLASS_IN_USE),   // sharing / lock / sharing-buffer violation
            39 | 112 => Some(ERROR_CLASS_DISK_FULL),    // too-many-sectors / disk full
            _ => None,
        }
    }
    #[cfg(not(windows))]
    {
        match code {
            1 | 13 => Some(ERROR_CLASS_ACCESS_DENIED), // EPERM / EACCES
            16 | 26 => Some(ERROR_CLASS_IN_USE),       // EBUSY / ETXTBSY
            28 | 69 => Some(ERROR_CLASS_DISK_FULL),    // ENOSPC / EDQUOT
            _ => None,
        }
    }
}

fn ok_paths(paths: Vec<String>) -> FileOpsResult {
    FileOpsResult {
        ok: true,
        paths,
        error: None,
        error_code: None,
        error_params: None,
        error_class: None,
        cancelled: false,
    }
}

fn err_msg(message: impl Into<String>) -> FileOpsResult {
    let message = message.into();
    let error_class = classify_transfer_error(&message);
    FileOpsResult {
        ok: false,
        paths: Vec::new(),
        error: Some(message),
        error_code: None,
        error_params: None,
        error_class,
        cancelled: false,
    }
}

/// Build an `ok = false` result carrying a machine-readable `error_code` plus
/// interpolated `error_params`. The English `message` is kept verbatim in
/// `error` for logs and for any locale that lacks the code (backward compat).
/// `error_class` still runs through [`classify_transfer_error`] so a wrapped OS
/// failure keeps surfacing the Retry / Skip / Abort class.
fn err_code(
    code: &str,
    params: Vec<(String, String)>,
    message: String,
) -> FileOpsResult {
    let error_class = classify_transfer_error(&message);
    let mut map = std::collections::HashMap::new();
    for (key, value) in params {
        map.insert(key, value);
    }
    FileOpsResult {
        ok: false,
        paths: Vec::new(),
        error: Some(message),
        error_code: Some(code.to_string()),
        error_params: if map.is_empty() {
            None
        } else {
            Some(map)
        },
        error_class,
        cancelled: false,
    }
}

/// Classify a known English error message (emitted by the `Result<_, String>`
/// helpers) into a localized-error code + params. This lets the `file_ops_*`
/// command wrappers tag codes **without** threading a code through every helper
/// (minimal-touch): the helpers stay `Result<_, String>`, and the wrapper maps
/// the string back to a code at the boundary. Returns `None` for hard refusals
/// (folder-into-itself, volume root) and anything unrecognized, so those keep
/// the raw `error` fallback in the frontend and never surface a misleading
/// `fileOps.error.*` line. `params` is empty here: the codes this returns
/// (validation / path checks) carry no named placeholders.
fn classify_error(message: &str) -> Option<(&'static str, Vec<(String, String)>)> {
    if message.contains("Path does not exist") {
        return Some(("path_missing", Vec::new()));
    }
    if message.contains("Path is not a directory") {
        return Some(("not_a_directory", Vec::new()));
    }
    if message.contains("Name must not be empty") {
        return Some(("name_empty", Vec::new()));
    }
    if message.contains("Name contains invalid characters") {
        return Some(("name_invalid", Vec::new()));
    }
    if message.contains("Name is not allowed") {
        return Some(("name_not_allowed", Vec::new()));
    }
    // rename / create collision: `A file|folder named 'X' already exists`.
    // Extract the quoted name so the localized line can interpolate {name}.
    if message.contains("already exists") {
        let name = quoted_name(message);
        let params = if name.is_empty() {
            Vec::new()
        } else {
            vec![("name".to_string(), name)]
        };
        return Some(("name_exists", params));
    }
    // Recycle Bin list / restore with no host bin API (e.g. non-NTFS / macOS).
    if message.contains("Recycle Bin") && message.contains("unavailable") {
        return Some(("recycle_unavailable", Vec::new()));
    }
    // Generic copy / move IO failure surfaced at a command boundary.
    if message.starts_with("Failed to ") {
        return Some(("io_failed", Vec::new()));
    }
    None
}

/// Extract the single-quoted token from a message like `A file named 'x' already
/// exists`. Returns an empty string when there is no well-formed quoted segment.
fn quoted_name(message: &str) -> String {
    if let Some(start) = message.find('\'') {
        if let Some(end) = message[start + 1..].find('\'') {
            return message[start + 1..start + 1 + end].to_string();
        }
    }
    String::new()
}

/// Command-wrapper helper: attach a localized `error_code` when the English
/// message maps to a known code, otherwise fall back to the plain [`err_msg`]
/// (which still populates `error` + `error_class`). Behavior-preserving — it
/// only adds `error_code` / `error_params`.
fn err_msg_or_classified(message: impl Into<String>) -> FileOpsResult {
    let message = message.into();
    match classify_error(&message) {
        Some((code, params)) => err_code(code, params, message),
        None => err_msg(message),
    }
}

fn cancelled_result() -> FileOpsResult {
    FileOpsResult {
        ok: false,
        paths: Vec::new(),
        error: Some("Cancelled".to_string()),
        error_code: None,
        error_params: None,
        error_class: None,
        cancelled: true,
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

/// 删除用：目标不存在的损坏 junction/symlink 仍视为存在，以便只删链接项。
fn ensure_deletable(path: &str) -> Result<PathBuf, String> {
    assert_os_path(path)?;
    let pb = PathBuf::from(path.trim());
    if fs::symlink_metadata(&pb).is_ok() {
        return Ok(pb);
    }
    if pb.exists() {
        return Ok(pb);
    }
    Err(format!("Path does not exist: {path}"))
}

/// 路径是否为重解析点（junction / symlink），不跟随目标。
fn is_reparse_point(path: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        return meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// 只删除链接/重解析点本身，不进入、不删除目标内容。
fn unlink_reparse_point(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|err| format!("Failed to stat link for delete: {err}"))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
        if meta.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0 {
            // 目录 junction / 目录符号链接：RemoveDirectory 只拆链接。
            return fs::remove_dir(path)
                .map_err(|err| format!("Failed to delete directory link: {err}"));
        }
        return fs::remove_file(path).map_err(|err| format!("Failed to delete file link: {err}"));
    }
    #[cfg(not(windows))]
    {
        let _ = meta;
        fs::remove_file(path).map_err(|err| format!("Failed to delete link: {err}"))
    }
}

/// 永久删除：链接只卸链接；普通目录才 `remove_dir_all`。
fn delete_permanent_one(path: &Path) -> Result<(), String> {
    if is_reparse_point(path) {
        return unlink_reparse_point(path);
    }
    let meta = fs::metadata(path).map_err(|err| format!("Failed to permanently delete: {err}"))?;
    if meta.is_dir() {
        fs::remove_dir_all(path).map_err(|err| format!("Failed to permanently delete: {err}"))
    } else {
        fs::remove_file(path).map_err(|err| format!("Failed to permanently delete: {err}"))
    }
}

fn ensure_dir(path: &str) -> Result<PathBuf, String> {
    let pb = ensure_existing(path)?;
    if !pb.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }
    Ok(pb)
}

/// Windows 资源管理器 / Win32 保留名（不区分大小写、含点后缀，如 `CON.txt`、`nul.dat`）。
/// 仅 `COM1-9` / `LPT1-9` 命中；`COM0` / `LPT0` 不在传统保留列表，与 Explorer 行为一致。
#[cfg(windows)]
fn is_windows_reserved_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    if stem.len() == 4 {
        let (head, tail) = stem.split_at(3);
        let last = tail.as_bytes()[0];
        if (head == "COM" || head == "LPT") && (b'1'..=b'9').contains(&last) {
            return true;
        }
    }
    false
}

fn validate_file_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Name must not be empty".to_string());
    }
    if name == "." || name == ".." {
        return Err("Name is not allowed".to_string());
    }
    // POSIX only forbids '/' (and NUL, unreachable in a String). Backslash,
    // ':', '*', '?', '"', '<', '>', '|' and leading/trailing dots/spaces are
    // all legal in APFS/HFS+, so the Win32 normalization below is gated by
    // platform — otherwise names like `CON`, `a.` or `report:2026` are rejected
    // on macOS even though the filesystem accepts them.
    #[cfg(windows)]
    {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("Name must not be empty".to_string());
        }
        if trimmed.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
            return Err("Name contains invalid characters".to_string());
        }
        if is_windows_reserved_name(trimmed) {
            return Err("Name is reserved by Windows".to_string());
        }
        // NTFS / Win32 默认会去掉文件名尾部的点和空格。提前对齐，避免「CreateFile 成功
        // 但路径不可访问」的鬼影。"a..." → "a"，"a.b.  " → "a.b"。
        let normalized = trimmed.trim_end_matches(['.', ' ']).to_string();
        if normalized.is_empty() || normalized == "." || normalized == ".." {
            return Err("Name is not allowed".to_string());
        }
        Ok(normalized)
    }
    #[cfg(not(windows))]
    {
        if name.contains('/') {
            return Err("Name contains invalid characters".to_string());
        }
        Ok(name.to_string())
    }
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

/// Whether `path` can be moved to the Recycle Bin. Delegates to the platform
/// layer, which keys off the *filesystem* (only NTFS on Windows), not merely
/// off UNC — see docs/file-ops-scenario-baseline.md DEL-003.
pub fn path_can_use_recycle_bin(path: &str) -> bool {
    crate::platform::volume_supports_recycle_bin(path)
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
    /// True when the destination is an existing directory and the source is also a
    /// directory: contents are **merged** rather than the whole tree replaced
    /// (CF-007 / CF-008). Inner collisions then recurse through the same conflict logic.
    merge: bool,
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
        // 同目录粘贴自身（剪贴板复制 / Ctrl 拖放）：keep 生成「name - Copy」副本；
        // replace 对自身是无操作；skip 跳过。剪切移动不会走到这里（同父目录已提前跳过）。
        return match conflict {
            NameConflict::KeepBoth => Ok(Some(DestPlan {
                path: unique_dest_path(parent, file_name),
                replace: false,
                merge: false,
            })),
            NameConflict::Skip => Ok(None),
            NameConflict::Replace => Ok(Some(DestPlan {
                path: exact,
                replace: false,
                merge: false,
            })),
        };
    }
    if !exact.exists() {
        return Ok(Some(DestPlan {
            path: exact,
            replace: false,
            merge: false,
        }));
    }
    // 同名目录入已存在的同名目录 → 合并（CF-007 / CF-008），与 Windows Explorer 一致。
    // 顶层不再询问「保留两者 / 跳过 / 替换」，合并内部的同名文件才按 conflict 递归处理。
    if source.is_dir() && exact.is_dir() {
        return Ok(Some(DestPlan {
            path: exact,
            replace: false,
            merge: true,
        }));
    }
    match conflict {
        NameConflict::KeepBoth => Ok(Some(DestPlan {
            path: unique_dest_path(parent, file_name),
            replace: false,
            merge: false,
        })),
        NameConflict::Skip => Ok(None),
        NameConflict::Replace => Ok(Some(DestPlan {
            path: exact,
            replace: true,
            merge: false,
        })),
    }
}

/// Outcome of a merge pass: the destination paths created, plus whether it was
/// interrupted by a cancel. A cancelled merge is a **safe stop** — the caller
/// keeps the already-committed atomic entries and does not attempt to un-merge
/// (LC-005 scoped-rollback decision 2026-11-22).
struct MergeRun {
    created: Vec<String>,
    cancelled: bool,
}

/// Merge the contents of `src` (a directory) into an **existing** `dest_dir`
/// (also a directory). Each child of `src`:
/// - both dirs → recurse (CF-007 dir-into-dir merge)
/// - otherwise → `place_dest` decides replace/skip/keep-both per inner item, then
///   leaf copy/move via `safe_io` (CF-008 inner-collision recursion).
/// For a move, leaf children are moved (source leaf removed by `safe_move_entry`);
/// a fully-emptied source subtree is removed; a source dir that still holds skipped
/// items is left in place. Checks the cancel token between children; on cancel it
/// stops (no rollback — see MergeRun). Returns every newly created/moved destination
/// path (via MergeRun.created) so undo and post-op selection can act on exactly what
/// changed.
fn merge_directory_into(
    src: &Path,
    dest_dir: &Path,
    conflict: NameConflict,
    is_move: bool,
    emitter: Option<&progress::ProgressEmitter>,
    cancel: Option<&cancel::CancelToken>,
) -> Result<MergeRun, String> {
    let mut created = Vec::new();
    let read = fs::read_dir(src).map_err(|err| format!("Failed to read folder for merge: {err}"))?;
    for entry in read {
        if matches!(cancel, Some(token) if token.cancelled()) {
            return Ok(MergeRun { created, cancelled: true });
        }
        let child = entry.map_err(|err| format!("Failed to read folder entry: {err}"))?.path();
        let name = child
            .file_name()
            .ok_or_else(|| "Invalid entry name during merge".to_string())?
            .to_string_lossy()
            .into_owned();
        let child_dest = dest_dir.join(&name);
        if child.is_dir() && child_dest.is_dir() {
            let sub = merge_directory_into(&child, &child_dest, conflict, is_move, emitter, cancel)?;
            created.extend(sub.created);
            if sub.cancelled {
                return Ok(MergeRun { created, cancelled: true });
            }
            // After merging a subtree on a move, drop the (now empty) source subdir.
            // Non-empty (some inner items skipped) → leave it.
            if is_move && fs::read_dir(&child).map(|mut r| r.next().is_none()).unwrap_or(false) {
                let _ = fs::remove_dir(&child);
            }
            continue;
        }
        let Some(plan) = place_dest(dest_dir, &name, &child, conflict)? else {
            continue; // inner item skipped
        };
        let on_bytes = |count: u64| {
            if let Some(emitter) = emitter {
                emitter.on_bytes(count);
            }
        };
        if is_move {
            if emitter.is_some() {
                safe_io::safe_move_entry_with_progress(&child, &plan.path, plan.replace, Some(&on_bytes))?;
            } else {
                safe_io::safe_move_entry(&child, &plan.path, plan.replace)?;
            }
        } else if emitter.is_some() {
            safe_io::safe_copy_entry_with_progress(&child, &plan.path, plan.replace, Some(&on_bytes))?;
        } else {
            safe_io::safe_copy_entry(&child, &plan.path, plan.replace)?;
        }
        created.push(display_path(&plan.path));
    }
    Ok(MergeRun {
        created,
        cancelled: false,
    })
}

/// Rollback action for a cancelled **non-merge** transfer (LC-005 scoped
/// rollback). Merge subtrees are never un-merged — the caller safe-stops.
enum RollbackOp {
    /// A copy created this destination path → delete it.
    DeleteCreated(PathBuf),
    /// A move relocated `original_src` → `created_dest` → move it back.
    MoveBack {
        created_dest: PathBuf,
        original_src: PathBuf,
    },
}

fn run_rollback(ops: &[RollbackOp]) {
    for op in ops.iter().rev() {
        match op {
            RollbackOp::DeleteCreated(path) => {
                // Best-effort: the item was created by this very operation.
                let _ = delete_permanent_one(path);
            }
            RollbackOp::MoveBack {
                created_dest,
                original_src,
            } => {
                // Move back where it came from. If this fails the item still exists
                // at created_dest — safe_io never deletes a source before the copy
                // is verified, so no data is lost either way.
                let _ = safe_io::safe_move_entry(created_dest, original_src, false);
            }
        }
    }
}

struct TransferRun {
    created: Vec<String>,
    cancelled: bool,
}

pub fn copy_paths(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
) -> Result<Vec<String>, String> {
    copy_paths_inner(sources, dest_dir, conflict, None, None).map(|run| run.created)
}

/// 可取消复制（LC-005）：传入已注册的 cancel token，条目边界检查取消并回滚已复制项。
/// 同时按进度发射器上报字节进度（右下角任务列表条）。
fn copy_paths_cancellable(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
    emitter: &progress::ProgressEmitter,
    cancel: &cancel::CancelToken,
) -> Result<TransferRun, String> {
    copy_paths_inner(sources, dest_dir, conflict, Some(emitter), Some(cancel))
}

fn copy_paths_inner(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
    emitter: Option<&progress::ProgressEmitter>,
    cancel: Option<&cancel::CancelToken>,
) -> Result<TransferRun, String> {
    let dest = ensure_dir(dest_dir)?;
    let mut created = Vec::new();
    let mut rollback: Vec<RollbackOp> = Vec::new();
    for source in sources {
        if matches!(cancel, Some(token) if token.cancelled()) {
            run_rollback(&rollback);
            return Ok(TransferRun {
                created: Vec::new(),
                cancelled: true,
            });
        }
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
        // CF-007 / CF-008: pasting a folder onto an existing same-name folder merges.
        if plan.merge {
            let merged = merge_directory_into(&src, &plan.path, conflict, false, emitter, cancel)?;
            if merged.cancelled {
                // Safe stop for the merge; roll back this call's non-merge items only.
                run_rollback(&rollback);
                return Ok(TransferRun {
                    created: Vec::new(),
                    cancelled: true,
                });
            }
            created.extend(merged.created);
            continue;
        }
        match emitter {
            Some(emitter) => safe_io::safe_copy_entry_with_progress(
                &src,
                &plan.path,
                plan.replace,
                Some(&|count| emitter.on_bytes(count)),
            )?,
            None => safe_io::safe_copy_entry(&src, &plan.path, plan.replace)?,
        }
        created.push(display_path(&plan.path));
        rollback.push(RollbackOp::DeleteCreated(plan.path.clone()));
    }
    Ok(TransferRun {
        created,
        cancelled: false,
    })
}

/// 待复制/移动的总字节数（目录递归求和；失败按 0 计，前端显示为不确定进度）。
pub fn total_transfer_bytes(sources: &[String]) -> u64 {
    let mut total = 0u64;
    for source in sources {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !path.exists() {
            continue;
        }
        if path.is_file() {
            total += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            continue;
        }
        for entry in WalkDir::new(&path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

pub fn move_paths(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
) -> Result<Vec<String>, String> {
    move_paths_inner(sources, dest_dir, conflict, None, None).map(|run| run.created)
}

/// 可取消移动（LC-005）：已移动的项在取消时移回原位；合并子树安全停止不回滚。
/// 同时按进度发射器上报字节进度。
fn move_paths_cancellable(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
    emitter: &progress::ProgressEmitter,
    cancel: &cancel::CancelToken,
) -> Result<TransferRun, String> {
    move_paths_inner(sources, dest_dir, conflict, Some(emitter), Some(cancel))
}

fn move_paths_inner(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
    emitter: Option<&progress::ProgressEmitter>,
    cancel: Option<&cancel::CancelToken>,
) -> Result<TransferRun, String> {
    let dest = ensure_dir(dest_dir)?;
    let mut created = Vec::new();
    let mut rollback: Vec<RollbackOp> = Vec::new();
    for source in sources {
        if matches!(cancel, Some(token) if token.cancelled()) {
            run_rollback(&rollback);
            return Ok(TransferRun {
                created: Vec::new(),
                cancelled: true,
            });
        }
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
            created.push(display_path(&src));
            continue;
        }
        let Some(plan) = place_dest(&dest, &name, &src, conflict)? else {
            continue;
        };
        // CF-007 / CF-008: moving a folder onto an existing same-name folder merges
        // its contents; the source folder is removed once it is fully emptied.
        if plan.merge {
            let merged = merge_directory_into(&src, &plan.path, conflict, true, emitter, cancel)?;
            if merged.cancelled {
                // Safe stop for the merge subtree; roll back this call's plain moves.
                run_rollback(&rollback);
                return Ok(TransferRun {
                    created: Vec::new(),
                    cancelled: true,
                });
            }
            created.extend(merged.created);
            if fs::read_dir(&src).map(|mut r| r.next().is_none()).unwrap_or(false) {
                let _ = fs::remove_dir(&src);
            }
            continue;
        }
        match emitter {
            Some(emitter) => safe_io::safe_move_entry_with_progress(
                &src,
                &plan.path,
                plan.replace,
                Some(&|count| emitter.on_bytes(count)),
            )?,
            None => safe_io::safe_move_entry(&src, &plan.path, plan.replace)?,
        }
        created.push(display_path(&plan.path));
        rollback.push(RollbackOp::MoveBack {
            created_dest: plan.path.clone(),
            original_src: src.clone(),
        });
    }
    Ok(TransferRun {
        created,
        cancelled: false,
    })
}

pub fn rename_path(path: &str, new_name: &str, replace: bool) -> Result<String, String> {
    let src = ensure_existing(path)?;
    let name = validate_file_name(new_name)?;
    let parent = src
        .parent()
        .ok_or_else(|| "Cannot rename a root path".to_string())?;
    let dest = parent.join(&name);
    // NM-004: on a case-insensitive FS, `a.txt → A.txt` (or any case-only change)
    // points at the *same* entry — that is a legal case rename, not a collision.
    // Only a genuinely different existing entry is rejected. Byte equality alone is
    // not enough because `dest != src` is true for a case-only change; compare the
    // resolved path case-insensitively on Windows / case-sensitively elsewhere.
    if dest.exists() && !is_same_entry(&dest, &src) {
        // NM-002: Explorer lets the user Resolve a rename collision with
        // Replace / Skip; the caller re-invokes with `replace = true` after the
        // user picks Replace. Only an existing *file* can be replaced by a
        // rename — overwriting a directory this way is not an Explorer rename
        // affordance, so it stays refused.
        if !replace {
            return Err(format!("A file named '{name}' already exists"));
        }
        if dest.is_dir() {
            return Err(format!("A folder named '{name}' already exists"));
        }
        fs::remove_file(&dest).map_err(|err| format!("Failed to replace: {err}"))?;
    }
    fs::rename(&src, &dest).map_err(|err| format!("Failed to rename: {err}"))?;
    Ok(display_path(&dest))
}

/// True when `a` and `b` refer to the same directory entry. On Windows
/// (NTFS/exFAT/FAT) the file system is case-insensitive, so a case-insensitive
/// path comparison is the right notion of "same entry". On Unix, APFS/HFS+ can
/// be formatted either case-sensitive or case-insensitive, so a static answer
/// is wrong — compare (device, inode) identity instead. A case-only rename on
/// a case-insensitive volume resolves to the same inode (allowed); a genuinely
/// different file has a different inode (collision, must be refused).
fn is_same_entry(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        a.as_os_str().to_string_lossy().to_lowercase()
            == b.as_os_str().to_string_lossy().to_lowercase()
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(a), fs::metadata(b)) {
            (Ok(ma), Ok(mb)) => ma.dev() == mb.dev() && ma.ino() == mb.ino(),
            // Callers only invoke this when `dest.exists()`, so a metadata
            // failure is exceptional; fall back to a byte comparison.
            _ => a == b,
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        a == b
    }
}

pub fn create_directory(parent: &str, name: &str) -> Result<String, String> {
    let parent_pb = ensure_dir(parent)?;
    let folder_name = validate_file_name(name)?;
    let dest = parent_pb.join(&folder_name);
    if dest.exists() {
        return Err(format!("A file named '{folder_name}' already exists"));
    }
    fs::create_dir(&dest).map_err(|err| format!("Failed to create folder: {err}"))?;
    Ok(display_path(&dest))
}

pub fn create_empty_file(parent: &str, name: &str) -> Result<String, String> {
    let parent_pb = ensure_dir(parent)?;
    let file_name = validate_file_name(name)?;
    let dest = parent_pb.join(&file_name);
    if dest.exists() {
        return Err(format!("A file named '{file_name}' already exists"));
    }
    fs::File::create(&dest).map_err(|err| format!("Failed to create file: {err}"))?;
    Ok(display_path(&dest))
}

pub fn delete_paths(paths: &[String]) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();
    for path in paths {
        let pb = ensure_deletable(path)?;
        safe_io::refuse_volume_root(&pb).map_err(|err| {
            err.replace(
                "destructive op on a volume root",
                "move to Recycle Bin for a volume root",
            )
        })?;
        // 回收站路径必须是链接项本身；若 Shell 对损坏重解析点失败则直接卸链接。
        // 7.24: go through the registered trash provider instead of the OS
        // backend directly; `delete_to_recycle` remains the provider body.
        match crate::platform::delete_to_trash(&pb, path) {
            Ok(()) => {}
            Err(err) if is_reparse_point(&pb) => {
                unlink_reparse_point(&pb).map_err(|unlink_err| {
                    format!("{err}; link-only fallback also failed: {unlink_err}")
                })?;
            }
            Err(err) => return Err(err),
        }
        deleted.push(display_path(&pb));
    }
    Ok(deleted)
}

pub fn delete_permanent_paths(paths: &[String]) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();
    for path in paths {
        let pb = ensure_deletable(path)?;
        safe_io::refuse_volume_root(&pb).map_err(|err| {
            err.replace(
                "destructive op on a volume root",
                "permanently delete a volume root",
            )
        })?;
        delete_permanent_one(&pb)?;
        deleted.push(display_path(&pb));
    }
    Ok(deleted)
}

#[tauri::command]
pub fn file_ops_name_exists(dest_dir: String, name: String) -> Result<bool, String> {
    let dest = ensure_dir(&dest_dir)?;
    let file_name = validate_file_name(&name)?;
    Ok(dest.join(&file_name).exists())
}

/// TR-013 / CF-006: does the paste target already hold `name`, and is its type
/// (file vs directory) different from the source's? The FE uses `typeMismatch`
/// to show the "this replaces the folder / file" warning. A folder-into-folder
/// merge (CF-007/CF-008) is `typeMismatch = false` and not surfaced as a warning.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NameConflictInfo {
    pub exists: bool,
    pub type_mismatch: bool,
}

fn name_conflict_info(dest_dir: &str, name: &str, source: &str) -> Result<NameConflictInfo, String> {
    let dest = ensure_dir(dest_dir)?;
    let file_name = validate_file_name(name)?;
    let candidate = dest.join(&file_name);
    // Broken links still "exist" for conflict purposes (metadata via symlink path).
    let exists = candidate.exists() || fs::symlink_metadata(&candidate).is_ok();
    let type_mismatch = if exists {
        let src_is_dir = fs::metadata(source).map(|m| m.is_dir()).unwrap_or(false);
        let dst_is_dir = candidate.metadata().map(|m| m.is_dir()).unwrap_or(false);
        src_is_dir != dst_is_dir
    } else {
        false
    };
    Ok(NameConflictInfo {
        exists,
        type_mismatch,
    })
}

#[tauri::command]
pub fn file_ops_name_conflict_info(
    dest_dir: String,
    name: String,
    source: String,
) -> Result<NameConflictInfo, String> {
    name_conflict_info(&dest_dir, &name, &source)
}

/// UN-008: a cheap change-detection fingerprint for undo contamination checks.
/// We use size + last-modified (ms) rather than a content hash — O(1) stat, no
/// file read, good enough to catch "the user edited/overwrote the created file
/// in another program after the operation, before Ctrl+Z".
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntryFingerprint {
    pub size: u64,
    pub modified_ms: u64,
}

fn fingerprint_one(path: &str) -> Option<EntryFingerprint> {
    let meta = fs::metadata(path.trim()).ok()?;
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Some(EntryFingerprint {
        size: meta.len(),
        modified_ms,
    })
}

/// Returns one fingerprint per input path; `None` where the path is gone or
/// unreadable (treated as "contaminated" by the caller — never blindly delete).
#[tauri::command]
pub fn file_ops_fingerprints(paths: Vec<String>) -> Vec<Option<EntryFingerprint>> {
    paths.iter().map(|p| fingerprint_one(p)).collect()
}

/// 大目录复制不得占用 Tauri 同步命令线程。
/// 传 taskId 时按字节进度发射 `file-ops-progress` 事件（右下角任务列表条），
/// 并注册 cancel token：用户取消时在条目边界停止并回滚已复制项（LC-005）。
#[tauri::command]
pub async fn file_ops_copy(
    app: tauri::AppHandle,
    sources: Vec<String>,
    dest_dir: String,
    conflict: Option<String>,
    task_id: Option<String>,
) -> FileOpsResult {
    let mode = parse_name_conflict(conflict.as_deref());
    let tracked_task_id = task_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    match tauri::async_runtime::spawn_blocking(move || match tracked_task_id {
        Some(task_id) => {
            let total = total_transfer_bytes(&sources);
            let cancel = cancel::register(&task_id);
            let emitter = progress::ProgressEmitter::new(app, task_id, total);
            emitter.emit_start();
            let outcome = copy_paths_cancellable(&sources, &dest_dir, mode, &emitter, &cancel);
            match &outcome {
                Ok(run) if run.cancelled => emitter.emit_done(true, Some("Cancelled".into())),
                Ok(_) => emitter.emit_done(true, None),
                Err(error) => emitter.emit_done(false, Some(error.clone())),
            }
            outcome
        }
        None => copy_paths(&sources, &dest_dir, mode)
            .map(|created| TransferRun { created, cancelled: false }),
    })
    .await
    {
        Ok(Ok(run)) if run.cancelled => cancelled_result(),
        Ok(Ok(run)) => ok_paths(run.created),
        Ok(Err(error)) => err_msg_or_classified(error),
        Err(error) => err_msg(format!("file_ops_copy worker failed: {error}")),
    }
}

/// 大目录移动不得占用 Tauri 同步命令线程。
/// 传 taskId 时上报进度（同卷 rename 瞬时完成，跨卷复制阶段有字节进度），
/// 并注册 cancel token：取消时把已移动项移回原位（LC-005）。
#[tauri::command]
pub async fn file_ops_move(
    app: tauri::AppHandle,
    sources: Vec<String>,
    dest_dir: String,
    conflict: Option<String>,
    task_id: Option<String>,
) -> FileOpsResult {
    let mode = parse_name_conflict(conflict.as_deref());
    let tracked_task_id = task_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    match tauri::async_runtime::spawn_blocking(move || match tracked_task_id {
        Some(task_id) => {
            let total = total_transfer_bytes(&sources);
            let cancel = cancel::register(&task_id);
            let emitter = progress::ProgressEmitter::new(app, task_id, total);
            emitter.emit_start();
            let outcome = move_paths_cancellable(&sources, &dest_dir, mode, &emitter, &cancel);
            match &outcome {
                Ok(run) if run.cancelled => emitter.emit_done(true, Some("Cancelled".into())),
                Ok(_) => emitter.emit_done(true, None),
                Err(error) => emitter.emit_done(false, Some(error.clone())),
            }
            outcome
        }
        None => move_paths(&sources, &dest_dir, mode)
            .map(|created| TransferRun { created, cancelled: false }),
    })
    .await
    {
        Ok(Ok(run)) if run.cancelled => cancelled_result(),
        Ok(Ok(run)) => ok_paths(run.created),
        Ok(Err(error)) => err_msg_or_classified(error),
        Err(error) => err_msg(format!("file_ops_move worker failed: {error}")),
    }
}

/// LC-005: user pressed Cancel on a running copy/move long task. Returns true
/// when a matching in-flight task accepted the cancel (false if already done).
#[tauri::command]
pub fn file_ops_cancel(task_id: String) -> bool {
    cancel::request_cancel(task_id.trim())
}

#[tauri::command]
pub fn file_ops_rename(path: String, new_name: String, replace: Option<bool>) -> FileOpsResult {
    match rename_path(&path, &new_name, replace.unwrap_or(false)) {
        Ok(p) => ok_paths(vec![p]),
        Err(error) => err_msg_or_classified(error),
    }
}

#[tauri::command]
pub fn file_ops_create_dir(parent: String, name: String) -> FileOpsResult {
    match create_directory(&parent, &name) {
        Ok(p) => ok_paths(vec![p]),
        Err(error) => err_msg_or_classified(error),
    }
}

#[tauri::command]
pub fn file_ops_create_file(parent: String, name: String) -> FileOpsResult {
    match create_empty_file(&parent, &name) {
        Ok(p) => ok_paths(vec![p]),
        Err(error) => err_msg_or_classified(error),
    }
}

/// 多子文件目录移入回收站不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn file_ops_delete(paths: Vec<String>) -> FileOpsResult {
    match tauri::async_runtime::spawn_blocking(move || delete_paths(&paths)).await {
        Ok(Ok(paths)) => ok_paths(paths),
        Ok(Err(error)) => err_msg_or_classified(error),
        Err(error) => err_msg(format!("file_ops_delete worker failed: {error}")),
    }
}

/// 永久删除大目录不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn file_ops_delete_permanent(paths: Vec<String>) -> FileOpsResult {
    match tauri::async_runtime::spawn_blocking(move || delete_permanent_paths(&paths)).await {
        Ok(Ok(paths)) => ok_paths(paths),
        Ok(Err(error)) => err_msg_or_classified(error),
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
pub fn file_ops_recycle_list() -> Result<Vec<crate::platform::RecycleBinItem>, String> {
    crate::platform::trash_list()
}

/// 批量从回收站还原不得占用 Tauri 同步命令线程（多文件撤销删除会扫回收站）。
#[tauri::command]
pub async fn file_ops_restore(paths: Vec<String>) -> FileOpsResult {
    match tauri::async_runtime::spawn_blocking(move || crate::platform::trash_restore(&paths)).await
    {
        Ok(Ok(restored)) => ok_paths(restored),
        Ok(Err(error)) => err_msg_or_classified(error),
        Err(error) => err_msg(format!("file_ops_restore worker failed: {error}")),
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
        let dir =
            std::env::temp_dir().join(format!("afm-file-ops-{}-{}", label, std::process::id()));
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

    // CF-005: 在大小写不敏感 FS（NTFS/exFAT/FAT、macOS 默认 APFS）上，`Foo.txt` 与
    // `foo.txt` 视为同一名字。`unique_dest_path` 通过 `Path::exists()` 走 OS 校验，
    // 自动按 FS 规则处理；本测试钉住该行为，防止后续被改成大小写敏感的纯字符串比较。
    #[cfg(windows)]
    #[test]
    fn unique_dest_treats_case_only_difference_as_collision() {
        let dir = temp_workspace("unique-case");
        fs::write(dir.join("Foo.txt"), b"original").unwrap();
        // 粘贴 foo.txt（小写）时，目标目录里 Foo.txt 已占用同名 → 必须返回 `foo - Copy.txt`，
        // 不是 `foo.txt`（FS 会拒绝）。
        let first = unique_dest_path(&dir, "foo.txt");
        assert_eq!(
            first.file_name().unwrap(),
            "foo - Copy.txt",
            "case-only difference must collide on case-insensitive FS"
        );
        // 已存在 `foo - Copy.txt` 时再次粘贴 FOO.TXT（不同大小写）应继续往后排。
        // stem=FOO / ext=.TXT，第一次 `FOO - Copy.TXT` 与 `foo - Copy.txt` 在 NTFS 上同名 → n=2。
        fs::write(&first, b"copy").unwrap();
        let second = unique_dest_path(&dir, "FOO.TXT");
        assert_eq!(second.file_name().unwrap(), "FOO - Copy (2).TXT");
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

        let renamed = rename_path(&copied[0], "renamed.txt", false).unwrap();
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
    fn same_dir_paste_keep_creates_copy() {
        let dir = temp_workspace("self-copy");
        fs::write(dir.join("a.txt"), b"1").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("n.txt"), b"x").unwrap();

        // 同目录粘贴文件：keep 生成「a - Copy.txt」，原件保留。
        let copied = copy_paths(
            &[display_path(&dir.join("a.txt"))],
            &display_path(&dir),
            NameConflict::KeepBoth,
        )
        .unwrap();
        assert_eq!(copied.len(), 1);
        assert_eq!(Path::new(&copied[0]).file_name().unwrap(), "a - Copy.txt");
        assert!(Path::new(&copied[0]).is_file());
        assert_eq!(fs::read(dir.join("a.txt")).unwrap(), b"1");
        assert!(fs::read(&copied[0]).map(|b| b == b"1").unwrap_or(false));

        // 同目录粘贴文件夹：keep 生成「sub - Copy」，内容完整。
        let copied_dir = copy_paths(
            &[display_path(&dir.join("sub"))],
            &display_path(&dir),
            NameConflict::KeepBoth,
        )
        .unwrap();
        assert_eq!(copied_dir.len(), 1);
        assert_eq!(Path::new(&copied_dir[0]).file_name().unwrap(), "sub - Copy");
        assert!(Path::new(&copied_dir[0]).join("n.txt").is_file());

        // skip：不产生任何新条目。
        let skipped = copy_paths(
            &[display_path(&dir.join("a.txt"))],
            &display_path(&dir),
            NameConflict::Skip,
        )
        .unwrap();
        assert!(skipped.is_empty());
        assert!(!dir.join("a - Copy (2).txt").exists());

        // replace（对自身）：无操作成功，不产生副本。
        let replaced = copy_paths(
            &[display_path(&dir.join("a.txt"))],
            &display_path(&dir),
            NameConflict::Replace,
        )
        .unwrap();
        assert_eq!(replaced.len(), 1);
        assert_eq!(Path::new(&replaced[0]).file_name().unwrap(), "a.txt");
        assert!(!dir.join("a - Copy (2).txt").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_to_recycle_local_file() {
        let dir = temp_workspace("del");
        let file = dir.join("gone.txt");
        fs::write(&file, b"bye").unwrap();
        let path = display_path(&file);
        assert!(path_can_use_recycle_bin(&path));
        let result = delete_paths(&[path]);
        if cfg!(windows) {
            result.unwrap();
            assert!(!file.exists());
        } else {
            // trash 5.2 moves items via Finder AppleScript on macOS, which needs
            // TCC Automation permission. Without it the delete is a *declared*
            // failure, not a silent success — accept only that explicit failure.
            match result {
                Ok(deleted) => {
                    assert_eq!(deleted.len(), 1);
                    assert!(!file.exists());
                }
                Err(err) => assert!(
                    err.contains("Failed to move to Recycle Bin"),
                    "unexpected delete failure: {err}"
                ),
            }
        }
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
        #[cfg(windows)]
        let volume_root = r"C:\".to_string();
        #[cfg(not(windows))]
        let volume_root = "/".to_string();
        let err = delete_permanent_paths(&[volume_root]).unwrap_err();
        assert!(err.contains("root"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn permanent_delete_junction_removes_link_only() {
        use std::process::Command;

        let dir = temp_workspace("junc-del");
        let real = dir.join("real-target");
        let parent = dir.join("skills");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&parent).unwrap();
        fs::write(real.join("keep-me.txt"), b"important").unwrap();
        let link = parent.join("stock-code-list");
        let real_s = fs::canonicalize(&real)
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J", &link.to_string_lossy(), &real_s])
            .status()
            .unwrap();
        assert!(status.success());

        let removed = delete_permanent_paths(&[display_path(&link)]).unwrap();
        assert_eq!(removed.len(), 1);
        assert!(
            fs::symlink_metadata(&link).is_err(),
            "junction entry must be gone"
        );
        assert!(real.is_dir(), "target directory must remain");
        assert_eq!(
            fs::read(real.join("keep-me.txt")).unwrap(),
            b"important",
            "target contents must remain"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn permanent_delete_broken_junction_still_unlinks() {
        use std::process::Command;

        let dir = temp_workspace("junc-broken");
        let real = dir.join("real-target");
        let parent = dir.join("skills");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&parent).unwrap();
        let link = parent.join("broken-link");
        let real_s = fs::canonicalize(&real)
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J", &link.to_string_lossy(), &real_s])
            .status()
            .unwrap();
        assert!(status.success());
        fs::remove_dir_all(&real).unwrap();
        assert!(
            fs::symlink_metadata(&link).is_ok(),
            "broken junction node must still exist"
        );
        assert!(!link.exists(), "exists() follows missing target");

        delete_permanent_paths(&[display_path(&link)]).unwrap();
        assert!(
            fs::symlink_metadata(&link).is_err(),
            "broken junction must be unlinked"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unc_cannot_recycle() {
        assert!(!path_can_use_recycle_bin(r"\\server\share\file.txt"));
    }

    #[test]
    fn rejects_awam_paths() {
        let dest = display_path(&std::env::temp_dir());
        let err = copy_paths(&["awam://x".into()], &dest, NameConflict::KeepBoth).unwrap_err();
        assert!(err.to_ascii_lowercase().contains("awam"));
    }

    #[test]
    fn validate_name_blocks_separators() {
        assert!(validate_file_name("a/b").is_err());
        assert!(validate_file_name("..").is_err());
        assert!(validate_file_name("").is_err());
    }

    // NM-003: Windows 资源管理器 / Win32 保留名一律拒绝，含点后缀、不区分大小写。
    // Win32-only rule: these names are perfectly legal on APFS/HFS+.
    #[cfg(windows)]
    #[test]
    fn validate_name_blocks_windows_reserved() {
        for blocked in [
            "CON", "con", "CoN", "PRN", "AUX", "NUL", "nul",
            "COM1", "com1", "COM9", "LPT1", "lpt3", "LPT9",
            "CON.txt", "nul.dat", "COM1.log", "LPT9.bin",
        ] {
            assert!(
                validate_file_name(blocked).is_err(),
                "reserved name must be rejected: {blocked}"
            );
        }
        // 仅前缀相似、不是保留名的应允许。
        for allowed in ["CONSOLE", "console.txt", "NULP", "nullop", "COMPUTER1", "LPT.txt", "COM0", "LPT0"] {
            assert!(
                validate_file_name(allowed).is_ok(),
                "near-miss must be allowed: {allowed}"
            );
        }
    }

    // NM-007: 尾点 / 尾空格静默 strip，与 Win32 CreateFile 行为一致。
    // Win32-only rule: macOS keeps trailing dots/spaces verbatim.
    #[cfg(windows)]
    #[test]
    fn validate_name_strips_trailing_dot_and_space() {
        assert_eq!(validate_file_name("a.txt.").unwrap(), "a.txt");
        assert_eq!(validate_file_name("a.txt  ").unwrap(), "a.txt");
        assert_eq!(validate_file_name("a. ").unwrap(), "a");
        assert_eq!(validate_file_name("report...").unwrap(), "report");
        // 中间点不动；前导点（隐藏 / dot-file 风格）保留。
        assert_eq!(validate_file_name(".hidden").unwrap(), ".hidden");
        assert_eq!(validate_file_name("a.b.c").unwrap(), "a.b.c");
        // 全是点 / 空格的输入应早返回错误，不能 strip 到空字符串。
        assert!(validate_file_name("...  ").is_err());
        assert!(validate_file_name("   ").is_err());
    }

    // macOS / APFS: Win32-only restrictions must NOT reject names that POSIX
    // allows. Only '/' and "." / ".." stay invalid cross-platform.
    #[cfg(not(windows))]
    #[test]
    fn validate_name_keeps_posix_legal_names() {
        for allowed in [
            "CON", "con", "nul.dat", "COM1.log", "LPT9.bin",
            "a.", "a.txt  ", "report...", "a:b", "a*b", "a?b", "a|b",
            "a\"b", "a<b", "a>b", "a\\b", "  ",
        ] {
            assert_eq!(
                validate_file_name(allowed).unwrap(),
                allowed,
                "POSIX-legal name must survive verbatim: {allowed}"
            );
        }
        assert!(validate_file_name("a/b").is_err());
        assert!(validate_file_name("..").is_err());
        assert!(validate_file_name("").is_err());
    }

    // CF-007: 把 src 目录合并进已存在的同名目录，无内层冲突时内容并入，目标目录名不变。
    #[test]
    fn copy_dir_into_existing_dir_merges_no_inner_collision() {
        let dir = temp_workspace("merge-plain");
        let src = dir.join("src");
        let dest_root = dir.join("dest");
        let dest_same = dest_root.join("src"); // already exists, same name
        fs::create_dir_all(src.join("inner")).unwrap();
        fs::write(src.join("a.txt"), b"new-a").unwrap();
        fs::write(src.join("inner").join("b.txt"), b"new-b").unwrap();
        fs::create_dir_all(&dest_same).unwrap();
        fs::write(dest_same.join("keep.txt"), b"existing").unwrap();

        let merged = copy_paths(
            &[display_path(&src)],
            &display_path(&dest_root),
            NameConflict::KeepBoth,
        )
        .unwrap();
        // merge 返回目标里新建/并入的顶层项：a.txt + inner 目录（inner 内的 b.txt 由
        // safe_copy_entry 整体递归复制，返回的是 inner 目录本身，不是它的孙子项）。
        assert!(merged.iter().any(|p| p.ends_with("a.txt")), "a.txt merged");
        assert!(
            merged.iter().any(|p| p.ends_with("inner")),
            "inner dir merged (got {merged:?})"
        );
        assert!(
            dest_same.join("inner").join("b.txt").is_file(),
            "inner/b.txt materialized on disk"
        );
        // 目标目录名没变成 `src - Copy`
        assert!(dest_same.is_dir(), "target dir reused for merge");
        assert!(!dest_root.join("src - Copy").exists(), "no - Copy dir created");
        // 原有文件保留、新文件并入
        assert_eq!(fs::read(dest_same.join("keep.txt")).unwrap(), b"existing");
        assert_eq!(fs::read(dest_same.join("a.txt")).unwrap(), b"new-a");
        // 源不变
        assert!(src.join("a.txt").is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    // CF-008: 目录合并时内层同名文件冲突，按 conflict 递归处理（replace / skip / keep-both）。
    // 场景：把 src 目录（含 x.txt）粘贴到 parent，parent/src 已存在（含旧 x.txt）。
    #[test]
    fn copy_dir_into_existing_dir_handles_inner_collisions() {
        let base = temp_workspace("merge-inner");
        fs::create_dir_all(&base).unwrap();

        // ── Replace 分支 ──
        let a = base.join("r");
        let src = a.join("src");
        let parent = a.join("parent");
        let existing_same = parent.join("src"); // dest/src 已存在（同名目录 → merge）
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&existing_same).unwrap();
        fs::write(src.join("x.txt"), b"new").unwrap();
        fs::write(existing_same.join("x.txt"), b"old").unwrap();
        copy_paths(
            &[display_path(&src)],
            &display_path(&parent),
            NameConflict::Replace,
        )
        .unwrap();
        assert_eq!(fs::read(existing_same.join("x.txt")).unwrap(), b"new");
        assert!(
            !existing_same.join("x - Copy.txt").exists(),
            "no - Copy generated for replace"
        );
        assert!(src.join("x.txt").is_file(), "copy: source untouched");

        // ── Skip 分支：目标同名保留原内容，非冲突新项仍并入 ──
        let b = base.join("s");
        let src = b.join("src");
        let parent = b.join("parent");
        let existing_same = parent.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&existing_same).unwrap();
        fs::write(src.join("collide.txt"), b"new").unwrap();
        fs::write(existing_same.join("collide.txt"), b"old").unwrap();
        fs::write(src.join("brand-new.txt"), b"n").unwrap();
        copy_paths(
            &[display_path(&src)],
            &display_path(&parent),
            NameConflict::Skip,
        )
        .unwrap();
        assert_eq!(
            fs::read(existing_same.join("collide.txt")).unwrap(),
            b"old",
            "skip keeps existing target content"
        );
        assert!(
            !existing_same.join("collide - Copy.txt").exists(),
            "skip never makes - Copy"
        );
        assert!(
            existing_same.join("brand-new.txt").is_file(),
            "non-conflicting items still merge"
        );

        // ── KeepBoth 分支：内层同名走 unique_dest_path ──
        let c = base.join("k");
        let src = c.join("src");
        let parent = c.join("parent");
        let existing_same = parent.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&existing_same).unwrap();
        fs::write(src.join("x.txt"), b"new").unwrap();
        fs::write(existing_same.join("x.txt"), b"old").unwrap();
        copy_paths(
            &[display_path(&src)],
            &display_path(&parent),
            NameConflict::KeepBoth,
        )
        .unwrap();
        assert_eq!(
            fs::read(existing_same.join("x.txt")).unwrap(),
            b"old",
            "original file intact under keep-both"
        );
        assert!(
            existing_same.join("x - Copy.txt").is_file(),
            "inner collision under KeepBoth produces - Copy"
        );
        assert_eq!(fs::read(existing_same.join("x - Copy.txt")).unwrap(), b"new");

        let _ = fs::remove_dir_all(&base);
    }

    // CF-007 + move 语义：把目录移动合并进同名目录，源目录搬空后消失。
    #[test]
    fn move_dir_into_existing_dir_empties_source() {
        let dir = temp_workspace("merge-move");
        let src = dir.join("a");
        let dst_root = dir.join("b");
        let dst_same = dst_root.join("a");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst_same).unwrap();
        fs::write(src.join("1.txt"), b"x").unwrap();
        fs::write(dst_same.join("2.txt"), b"y").unwrap();

        move_paths(
            &[display_path(&src)],
            &display_path(&dst_root),
            NameConflict::KeepBoth,
        )
        .unwrap();
        assert!(
            !src.exists(),
            "source folder gone after merge-move (fully emptied)"
        );
        assert!(dst_same.join("1.txt").is_file(), "1.txt moved into b/a/");
        assert!(dst_same.join("2.txt").is_file(), "existing 2.txt preserved");
        assert!(!dst_root.join("a - Copy").exists(), "no - Copy dir on merge");
        let _ = fs::remove_dir_all(&dir);
    }

    // CF-008 边界：目录移动时内层一项被 skip → 源目录非空，必须保留（不能 remove_dir）。
    #[test]
    fn move_merge_with_skip_preserves_source_when_not_empty() {
        let dir = temp_workspace("merge-move-skip");
        let src = dir.join("a");
        let dst_root = dir.join("b");
        let dst_same = dst_root.join("a");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst_same).unwrap();
        fs::write(src.join("x.txt"), b"new").unwrap();
        fs::write(dst_same.join("x.txt"), b"old").unwrap();

        move_paths(
            &[display_path(&src)],
            &display_path(&dst_root),
            NameConflict::Skip,
        )
        .unwrap();
        assert!(src.is_dir(), "source folder must remain when nothing was moved");
        assert_eq!(fs::read(dst_same.join("x.txt")).unwrap(), b"old", "dest not overwritten");
        let _ = fs::remove_dir_all(&dir);
    }

    // TR-013 / CF-006: type-conflict detection for the paste dialog.
    #[test]
    fn name_conflict_info_detects_type_mismatch() {
        let dir = temp_workspace("type-conflict");
        let parent = dir.join("parent");
        fs::create_dir_all(&parent).unwrap();
        // existing folder named "item"
        fs::create_dir_all(parent.join("item")).unwrap();

        // file source vs existing folder → mismatch
        let src_file = dir.join("item"); // same base name "item", this is a file
        fs::write(&src_file, b"f").unwrap();
        let info = name_conflict_info(&display_path(&parent), "item", &display_path(&src_file)).unwrap();
        assert!(info.exists, "target exists");
        assert!(info.type_mismatch, "file-vs-folder is a type mismatch");

        // folder source vs existing folder → no mismatch (it's a merge)
        let src_dir = dir.join("folder-item");
        fs::create_dir_all(&src_dir).unwrap();
        let info = name_conflict_info(&display_path(&parent), "item", &display_path(&src_dir)).unwrap();
        assert!(info.exists);
        assert!(!info.type_mismatch, "folder-into-folder is merge, not a mismatch");

        // name not present at all → no conflict, no mismatch
        let info = name_conflict_info(&display_path(&parent), "nope", &display_path(&src_file)).unwrap();
        assert!(!info.exists, "no target");
        assert!(!info.type_mismatch, "no target → no mismatch");

        let _ = fs::remove_dir_all(&dir);
    }

    // NM-002: rename collision refused without replace; Replace overwrites a file,
    // never a directory.
    #[test]
    fn rename_replace_overwrites_existing_file_only_when_allowed() {
        let dir = temp_workspace("rename-replace");
        fs::write(dir.join("a.txt"), b"A").unwrap();
        fs::write(dir.join("b.txt"), b"B").unwrap();
        // NM-002 default: a real collision is refused without `replace`.
        let err = rename_path(&display_path(&dir.join("a.txt")), "b.txt", false).unwrap_err();
        assert!(err.contains("already exists"));
        assert_eq!(fs::read(dir.join("b.txt")).unwrap(), b"B");
        // Replace (user chose Replace in the dialog): a.txt overwrites b.txt.
        let dest = rename_path(&display_path(&dir.join("a.txt")), "b.txt", true).unwrap();
        assert_eq!(dest, display_path(&dir.join("b.txt")));
        assert_eq!(fs::read(dir.join("b.txt")).unwrap(), b"A");
        assert!(!dir.join("a.txt").exists());
        // Replace never clobbers a directory.
        let sub = dir.join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(dir.join("c.txt"), b"C").unwrap();
        let err = rename_path(&display_path(&dir.join("c.txt")), "sub", true).unwrap_err();
        assert!(err.contains("already exists"));
        assert!(sub.is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    // UN-008: fingerprint reflects size+mtime and turns None for a missing path.
    #[test]
    fn fingerprint_captures_size_and_flags_missing() {
        let dir = temp_workspace("fingerprint");
        let file = dir.join("a.txt");
        fs::write(&file, b"12345").unwrap();
        let fp = fingerprint_one(&display_path(&file)).expect("present file fingerprints");
        assert_eq!(fp.size, 5);
        // A missing path yields None (the caller treats it as contaminated).
        assert!(fingerprint_one(&display_path(&dir.join("nope.txt"))).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    // NM-004: on a case-insensitive FS, a case-only rename must SUCCEED (not
    // "already exists"); renaming to a genuinely different existing file must fail.
    #[cfg(windows)]
    #[test]
    fn rename_case_only_succeeds_but_real_collision_fails() {
        let dir = temp_workspace("case-rename");
        fs::write(dir.join("a.txt"), b"x").unwrap();

        // a.txt → A.txt : same entry, case-only → allowed, file still present, one file.
        let renamed = rename_path(&display_path(&dir.join("a.txt")), "A.txt", false).unwrap();
        assert!(Path::new(&renamed).is_file());
        assert_eq!(fs::read(dir.join("A.txt")).unwrap(), b"x");
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            1,
            "case rename must not duplicate"
        );

        // now rename A.txt → b.txt where a DIFFERENT b.txt already exists → must fail.
        fs::write(dir.join("b.txt"), b"other").unwrap();
        let err = rename_path(&display_path(&dir.join("A.txt")), "B.txt", false).unwrap_err();
        assert!(
            err.contains("already exists"),
            "different existing entry (even case-only different base) must block: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    // NM-004 on Unix: APFS may be case-insensitive or case-sensitive. The
    // (dev, inode) comparison must allow a case-only rename on an insensitive
    // volume while still refusing a genuinely different existing file.
    #[cfg(unix)]
    #[test]
    fn rename_case_only_uses_inode_identity_on_unix() {
        let dir = temp_workspace("case-rename-unix");
        let lower = dir.join("a.txt");
        fs::write(&lower, b"x").unwrap();
        // is_same_entry identifies the same file through a differently-cased
        // spelling only when this volume is case-insensitive.
        let upper = dir.join("A.txt");
        let same = is_same_entry(&upper, &lower);
        let other = dir.join("b.txt");
        fs::write(&other, b"other").unwrap();
        assert!(!is_same_entry(&other, &lower), "different inodes are never the same entry");

        let renamed = rename_path(&display_path(&lower), "A.txt", false).unwrap();
        assert!(Path::new(&renamed).is_file());
        assert_eq!(fs::read(dir.join("A.txt")).unwrap(), b"x");
        // A real collision is still refused regardless of volume sensitivity.
        fs::write(dir.join("C.txt"), b"third").unwrap();
        let err = rename_path(&display_path(&dir.join("C.txt")), "A.txt", false).unwrap_err();
        assert!(err.contains("already exists"), "real collision must block: {err}");
        // Document the probe result without branching assertions: the invariant
        // is "same inode ⇒ case-only rename allowed", which both hold here.
        assert!(same || !upper.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    // LC-006 / LC-007 / LC-008 — per-item error class, locale-stable via the raw
    // OS code Rust appends to io::Error Display. Code tables are platform-specific.
    #[cfg(windows)]
    #[test]
    fn classify_transfer_error_maps_windows_codes() {
        // in-use: sharing / lock violation, even with a Chinese OS message body.
        assert_eq!(
            classify_transfer_error("Failed to move: 另一个程序正在使用此文件 (os error 32)"),
            Some(ERROR_CLASS_IN_USE.to_string())
        );
        assert_eq!(
            classify_transfer_error("Failed to park existing destination: (os error 33)"),
            Some(ERROR_CLASS_IN_USE.to_string())
        );
        // access denied.
        assert_eq!(
            classify_transfer_error("Failed to create destination file: 拒绝访问。 (os error 5)"),
            Some(ERROR_CLASS_ACCESS_DENIED.to_string())
        );
        // disk full.
        assert_eq!(
            classify_transfer_error("Failed to write destination file: 磁盘空间不足。 (os error 112)"),
            Some(ERROR_CLASS_DISK_FULL.to_string())
        );
        // portable English fragments when no code is embedded.
        assert_eq!(
            classify_transfer_error("There is not enough disk space to complete."),
            Some(ERROR_CLASS_DISK_FULL.to_string())
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn classify_transfer_error_maps_posix_errnos() {
        // macOS / Linux errno: same classes, different numeric codes.
        assert_eq!(
            classify_transfer_error("Failed to move file: resource busy (os error 16)"),
            Some(ERROR_CLASS_IN_USE.to_string())
        );
        assert_eq!(
            classify_transfer_error("Failed to replace: text file busy (os error 26)"),
            Some(ERROR_CLASS_IN_USE.to_string())
        );
        assert_eq!(
            classify_transfer_error("Failed to write: permission denied (os error 13)"),
            Some(ERROR_CLASS_ACCESS_DENIED.to_string())
        );
        assert_eq!(
            classify_transfer_error("Failed to write: operation not permitted (os error 1)"),
            Some(ERROR_CLASS_ACCESS_DENIED.to_string())
        );
        assert_eq!(
            classify_transfer_error("Failed to write: no space left on device (os error 28)"),
            Some(ERROR_CLASS_DISK_FULL.to_string())
        );
        assert_eq!(
            classify_transfer_error("Failed to write: disk quota exceeded (os error 69)"),
            Some(ERROR_CLASS_DISK_FULL.to_string())
        );
        // An unrecognized code must NOT short-circuit the English fallback.
        // (This was the macOS regression: EBUSY/ENOSPC hit `_ => return None`.)
        assert_eq!(
            classify_transfer_error("There is not enough disk space to complete. (os error 999)"),
            Some(ERROR_CLASS_DISK_FULL.to_string())
        );
    }

    #[test]
    fn classify_transfer_error_leaves_hard_refusals_uncategorized() {
        // Folder-into-itself / volume-root / validation are NOT per-item retryable:
        // the frontend must keep aborting the whole batch for these, not show a
        // Retry / Skip / Abort dialog.
        assert_eq!(
            classify_transfer_error("Cannot copy a folder into itself: C:\\a"),
            None
        );
        assert_eq!(
            classify_transfer_error("Refusing destructive op on a volume root: C:\\"),
            None
        );
        assert_eq!(
            classify_transfer_error("Path does not exist: C:\\gone.txt"),
            None
        );
    }

    #[test]
    fn err_msg_populates_per_item_error_class() {
        // End-to-end wiring: an in-use move failure surfaced through err_msg carries
        // the class the FE dialog keys on, while a refusal carries none.
        let r = err_msg("Failed to move file: resource busy");
        assert!(!r.ok);
        assert_eq!(r.error_class.as_deref(), Some(ERROR_CLASS_IN_USE));

        let refusal = err_msg("Cannot move a folder into itself: D:\\a");
        assert_eq!(refusal.error_class, None);

        // Successes never carry a class.
        assert_eq!(ok_paths(vec!["x".to_string()]).error_class, None);
    }

    // LC-ERR: classify_error maps the known English helper messages to codes and
    // leaves hard refusals / unknowns untagged (None) so the FE keeps the raw
    // error fallback.
    #[test]
    fn classify_error_maps_known_messages_to_codes() {
        assert_eq!(
            classify_error("Path does not exist: C:\\gone.txt").map(|c| c.0),
            Some("path_missing")
        );
        assert_eq!(
            classify_error("Path is not a directory: C:\\file.txt").map(|c| c.0),
            Some("not_a_directory")
        );
        assert_eq!(
            classify_error("Name must not be empty").map(|c| c.0),
            Some("name_empty")
        );
        assert_eq!(
            classify_error("Name contains invalid characters").map(|c| c.0),
            Some("name_invalid")
        );
        assert_eq!(
            classify_error("Name is not allowed").map(|c| c.0),
            Some("name_not_allowed")
        );
        assert_eq!(
            classify_error("Failed to create folder: ...").map(|c| c.0),
            Some("io_failed")
        );
        assert_eq!(
            classify_error(
                "Recycle Bin restore is unavailable on macOS; use Finder's Trash instead"
            )
            .map(|c| c.0),
            Some("recycle_unavailable")
        );
        // A name collision carries an interpolated {name} extracted from quotes.
        let (code, params) =
            classify_error("A file named 'report.pdf' already exists").expect("name_exists");
        assert_eq!(code, "name_exists");
        assert_eq!(params, vec![("name".to_string(), "report.pdf".to_string())]);
    }

    #[test]
    fn classify_error_leaves_hard_refusals_untagged() {
        // folder-into-itself / volume root / Cancelled are NOT a localized code.
        assert!(classify_error("Cannot copy a folder into itself: C:\\a").is_none());
        assert!(classify_error("Refusing destructive op on a volume root: C:\\").is_none());
        assert!(classify_error("Cancelled").is_none());
        assert!(classify_error("Clipboard is empty").is_none());
    }

    #[test]
    fn quoted_name_extracts_single_token() {
        assert_eq!(quoted_name("A file named 'x' already exists"), "x");
        assert_eq!(quoted_name("A folder named 'my docs' already exists"), "my docs");
        assert_eq!(quoted_name("no quotes here"), "");
    }

    #[test]
    fn err_code_preserves_message_and_populates_fields() {
        let r = err_code(
            "name_exists",
            vec![("name".to_string(), "a.txt".to_string())],
            "A file named 'a.txt' already exists".to_string(),
        );
        assert!(!r.ok);
        assert_eq!(r.error_code.as_deref(), Some("name_exists"));
        assert_eq!(
            r.error_params.as_ref().and_then(|m| m.get("name")).map(String::as_str),
            Some("a.txt")
        );
        // The English message is kept verbatim for logs + unknown codes.
        assert_eq!(r.error.as_deref(), Some("A file named 'a.txt' already exists"));

        // No params → error_params is None (not an empty map).
        let no_params = err_code("path_missing", Vec::new(), "Path does not exist: x".to_string());
        assert!(no_params.error_params.is_none());
    }

    #[test]
    fn err_msg_or_classified_tags_known_and_falls_back() {
        let known = err_msg_or_classified("Path does not exist: C:\\gone.txt".to_string());
        assert_eq!(known.error_code.as_deref(), Some("path_missing"));
        assert_eq!(known.error.as_deref(), Some("Path does not exist: C:\\gone.txt"));

        // Unknown message → no code, but error_class still resolves via transfer path.
        let inuse = err_msg_or_classified("Failed to move file: resource busy");
        assert_eq!(inuse.error_code.as_deref(), Some("io_failed"));
        assert_eq!(inuse.error_class.as_deref(), Some(ERROR_CLASS_IN_USE));

        let refusal = err_msg_or_classified("Cannot move a folder into itself: D:\\a");
        assert!(refusal.error_code.is_none());
        assert!(refusal.error_params.is_none());
    }

    #[test]
    fn err_code_untagged_paths_keep_none() {
        // The plain helpers stay untagged; codes are attached at the boundary.
        assert!(err_msg("something odd").error_code.is_none());
        assert!(ok_paths(vec!["x".to_string()]).error_code.is_none());
    }

    // LC-006: copying OUT of a file that another process still has open (a held read
    // handle) must SUCCEED — a read-only copy is not blocked by the in-use lock — and
    // leave the source intact. Rust opens files with FILE_SHARE_READ, so the concurrent
    // handle models the "in use" scenario without needing an external locker.
    #[cfg(windows)]
    #[test]
    fn copy_paths_succeeds_while_source_handle_is_open() {
        let dir = temp_workspace("copy-inuse");
        let src_dir = dir.join("src");
        let dest = dir.join("dest");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dest).unwrap();
        let file = src_dir.join("note.txt");
        fs::write(&file, b"payload").unwrap();

        // Hold the source open while copying it out.
        let _held = fs::File::open(&file).unwrap();
        let copied = copy_paths(&[display_path(&file)], &display_path(&dest), NameConflict::KeepBoth)
            .expect("copy-out of an in-use file must succeed");
        assert_eq!(copied.len(), 1);
        assert!(Path::new(&copied[0]).is_file());
        assert!(file.exists(), "source stays intact after copy");
        assert_eq!(fs::read(&file).unwrap(), b"payload");
        let _ = fs::remove_dir_all(&dir);
    }

    // CLIP-008: a move whose source vanished externally (cut → deleted elsewhere →
    // paste) must fail with the `path_missing` code so the frontend skips the item
    // silently instead of erroring the whole batch. Not an error_class — no dialog.
    #[test]
    fn move_missing_source_tags_path_missing_for_clip008() {
        let dir = temp_workspace("clip008-missing");
        let dst = dir.join("dst");
        fs::create_dir_all(&dst).unwrap();
        let err = move_paths(
            &[display_path(&dir.join("gone.txt"))],
            &display_path(&dst),
            NameConflict::KeepBoth,
        )
        .expect_err("moving a vanished source must fail");
        let tagged = err_msg_or_classified(err);
        assert_eq!(tagged.error_code.as_deref(), Some("path_missing"));
        assert!(
            tagged.error_class.is_none(),
            "a vanished source is not a dialog (retry/skip/abort) class"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // LC-005: a pre-cancelled token makes the copy loop bail at the first
    // source boundary — no destination is created, result reports cancelled.
    #[test]
    fn cancelled_copy_short_circuits_before_work() {
        let dir = temp_workspace("cancel-copy");
        let src = dir.join("src");
        let dst = dir.join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("a.txt"), b"x").unwrap();

        let token = cancel::register("cancel-copy-token");
        assert!(cancel::request_cancel("cancel-copy-token"));
        let run = copy_paths_inner(
            &[display_path(&src.join("a.txt"))],
            &display_path(&dst),
            NameConflict::KeepBoth,
            None,
            Some(&token),
        )
        .unwrap();
        assert!(run.cancelled, "pre-cancelled token → cancelled");
        assert!(run.created.is_empty());
        assert!(!dst.join("a.txt").exists(), "nothing should have been copied");
        assert!(src.join("a.txt").is_file(), "source untouched");
        let _ = fs::remove_dir_all(&dir);
    }

    // LC-005 rollback primitive: a committed copy is undone by deleting the
    // created destination (source stays).
    #[test]
    fn rollback_removes_created_copy() {
        let dir = temp_workspace("rollback-copy");
        let dst = dir.join("dst");
        fs::create_dir_all(&dst).unwrap();
        let created = dst.join("a.txt");
        fs::write(&created, b"x").unwrap();

        run_rollback(&[RollbackOp::DeleteCreated(created.clone())]);
        assert!(!created.exists(), "created copy removed on rollback");
        let _ = fs::remove_dir_all(&dir);
    }

    // LC-005 rollback primitive: a committed plain move is undone by moving the
    // item back to where it came from.
    #[test]
    fn rollback_moves_back_a_plain_move() {
        let dir = temp_workspace("rollback-move");
        let src_root = dir.join("src");
        let dst_root = dir.join("dst");
        fs::create_dir_all(&src_root).unwrap();
        fs::create_dir_all(&dst_root).unwrap();
        // Simulate an already-moved file: it lives at dst, absent at src.
        let dest_file = dst_root.join("doc.txt");
        let orig_file = src_root.join("doc.txt");
        fs::write(&dest_file, b"payload").unwrap();

        run_rollback(&[RollbackOp::MoveBack {
            created_dest: dest_file.clone(),
            original_src: orig_file.clone(),
        }]);
        assert!(!dest_file.exists(), "rolled-back file leaves the destination");
        assert!(orig_file.is_file(), "file restored to its original path");
        assert_eq!(fs::read(&orig_file).unwrap(), b"payload");
        let _ = fs::remove_dir_all(&dir);
    }
}
