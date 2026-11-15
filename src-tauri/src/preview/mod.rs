// SPDX-License-Identifier: GPL-3.0-or-later
//! Experimental image + plain-text preview for Navigator.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_TEXT_BYTES: u64 = 256 * 1024;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

/// Link metadata (`kind: "link"` payloads): what a preview needs to know
/// about a shortcut / reparse point instead of "file not found".
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLinkInfo {
    /// `"shortcut"` (.lnk) or `"symlink"` (junction / symbolic link).
    pub kind: String,
    /// Resolved target path (`None` when the link carries no readable target).
    pub target: Option<String>,
    /// Whether the target still exists on disk.
    pub target_exists: bool,
    /// `"dir"` / `"file"` of the target (`None` when missing or unknown).
    pub target_kind: Option<String>,
    /// Directory containing the target (原目录 / start-in folder).
    pub target_dir: Option<String>,
    /// Whether the target's parent directory (原目录) still exists.
    pub target_dir_exists: bool,
}

/// Directory preview (`kind: "dir"` payloads): lightweight folder basics
/// (direct children + timestamps) instead of a "Not a file" error.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDirInfo {
    /// Direct child files (`None` when the folder cannot be read).
    pub file_count: Option<u64>,
    /// Direct child subfolders (`None` when the folder cannot be read).
    pub dir_count: Option<u64>,
    pub modified_ms: Option<u64>,
    pub created_ms: Option<u64>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPayload {
    pub kind: String,
    pub path: String,
    pub text: Option<String>,
    pub data_url: Option<String>,
    pub truncated: bool,
    pub error: Option<String>,
    /// Link metadata; `Some` only for `kind: "link"` payloads.
    pub link: Option<PreviewLinkInfo>,
    /// Directory basics; `Some` only for `kind: "dir"` payloads.
    pub dir: Option<PreviewDirInfo>,
}

fn system_time_ms(time: std::time::SystemTime) -> Option<u64> {
    time.duration_since(std::time::SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico"
    )
}

fn is_text_ext(ext: &str) -> bool {
    matches!(
        ext,
        "txt"
            | "md"
            | "markdown"
            | "json"
            | "jsonc"
            | "csv"
            | "tsv"
            | "log"
            | "xml"
            | "html"
            | "htm"
            | "css"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "vue"
            | "rs"
            | "py"
            | "toml"
            | "yaml"
            | "yml"
            | "ini"
            | "cfg"
            | "conf"
            | "gitignore"
            | "env"
            | "svg"
    )
}

fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn unsupported(path: &str, truncated: bool, error: Option<String>) -> PreviewPayload {
    PreviewPayload {
        kind: "unsupported".into(),
        path: path.to_string(),
        text: None,
        data_url: None,
        truncated,
        error,
        link: None,
        dir: None,
    }
}

fn preview_link(path: &str, link: PreviewLinkInfo) -> PreviewPayload {
    PreviewPayload {
        kind: "link".into(),
        path: path.to_string(),
        text: None,
        data_url: None,
        truncated: false,
        error: None,
        link: Some(link),
        dir: None,
    }
}

fn preview_dir(path: &str, info: PreviewDirInfo) -> PreviewPayload {
    PreviewPayload {
        kind: "dir".into(),
        path: path.to_string(),
        text: None,
        data_url: None,
        truncated: false,
        error: None,
        link: None,
        dir: Some(info),
    }
}

/// Lightweight directory basics for the preview pane: one non-recursive
/// `read_dir` pass for direct child counts plus the folder's timestamps.
fn dir_info(path: &Path) -> PreviewDirInfo {
    let meta = fs::metadata(path).ok();
    let (file_count, dir_count) = match fs::read_dir(path) {
        Ok(entries) => {
            let mut files = 0u64;
            let mut dirs = 0u64;
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    dirs = dirs.saturating_add(1);
                } else {
                    files = files.saturating_add(1);
                }
            }
            (Some(files), Some(dirs))
        }
        Err(_) => (None, None),
    };
    PreviewDirInfo {
        file_count,
        dir_count,
        modified_ms: meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(system_time_ms),
        created_ms: meta
            .as_ref()
            .and_then(|m| m.created().ok())
            .and_then(system_time_ms),
    }
}

/// Describe a link's target: existence, kind, and the directory it lives in.
pub(crate) fn describe_link_target(kind: &str, target: &Path) -> PreviewLinkInfo {
    let (target_exists, target_kind) = match fs::metadata(target) {
        Ok(meta) => (
            true,
            Some(if meta.is_dir() { "dir" } else { "file" }.to_string()),
        ),
        Err(_) => (false, None),
    };
    let target_dir = target
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned());
    let target_dir_exists = target
        .parent()
        .map(|parent| fs::metadata(parent).map(|meta| meta.is_dir()).unwrap_or(false))
        .unwrap_or(false);
    PreviewLinkInfo {
        kind: kind.to_string(),
        target: Some(target.to_string_lossy().into_owned()),
        target_exists,
        target_kind,
        target_dir,
        target_dir_exists,
    }
}

/// Strip `\\?\` / `\??\` prefixes from a raw reparse target.
fn clean_reparse_target(raw: &Path) -> PathBuf {
    let text = raw.to_string_lossy();
    let cleaned = text
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| text.strip_prefix(r"\\?\").map(|rest| rest.to_string()))
        .or_else(|| text.strip_prefix(r"\??\").map(|rest| rest.to_string()))
        .unwrap_or_else(|| text.to_string());
    PathBuf::from(cleaned)
}

/// Link info for any reparse point (junction / symbolic link), valid or broken.
/// Returns `None` for regular files and directories. Shared by the preview
/// pane and the 属性 panel (`navigator_stat`).
pub(crate) fn reparse_link_info(path: &Path) -> Option<PreviewLinkInfo> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_symlink() {
        return None;
    }
    let raw = fs::read_link(path).ok()?;
    let target = clean_reparse_target(&raw);
    let target = if target.is_absolute() {
        target
    } else {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&target)
    };
    Some(describe_link_target("symlink", &target))
}

/// Broken reparse points (junction / symlink whose target is gone) must preview
/// as link info instead of a "file not found" style error. Valid links return
/// `None` so the normal preview follows their target.
fn broken_reparse_link_info(path: &Path) -> Option<PreviewLinkInfo> {
    let info = reparse_link_info(path)?;
    if info.target_exists {
        return None;
    }
    Some(info)
}

#[tauri::command]
pub fn preview_load(path: String) -> PreviewPayload {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return unsupported("", false, Some("Path is empty".into()));
    }
    let pb = Path::new(trimmed);
    if let Some(info) = broken_reparse_link_info(pb) {
        return preview_link(trimmed, info);
    }
    if !pb.is_file() {
        // Folders preview as basic info (children / timestamps), not an error.
        if pb.is_dir() {
            return preview_dir(trimmed, dir_info(pb));
        }
        return unsupported(trimmed, false, Some("Not a file".into()));
    }
    let ext = extension_lower(pb);
    // Windows shortcuts: preview the link's target info (target, 原目录 …),
    // never the opaque .lnk binary.
    if ext == "lnk" {
        return match crate::file_ops::shortcut::read_shortcut_target(pb) {
            Ok(target) if target.is_empty() => preview_link(
                trimmed,
                PreviewLinkInfo {
                    kind: "shortcut".into(),
                    target: None,
                    target_exists: false,
                    target_kind: None,
                    target_dir: None,
                    target_dir_exists: false,
                },
            ),
            Ok(target) => preview_link(
                trimmed,
                describe_link_target("shortcut", Path::new(&target)),
            ),
            Err(err) => unsupported(trimmed, false, Some(err)),
        };
    }
    let meta = match fs::metadata(pb) {
        Ok(m) => m,
        Err(err) => {
            return unsupported(trimmed, false, Some(format!("Failed to read metadata: {err}")));
        }
    };

    if is_image_ext(&ext) {
        if meta.len() > MAX_IMAGE_BYTES {
            return unsupported(
                trimmed,
                true,
                Some(format!(
                    "Image too large for preview (max {} MB)",
                    MAX_IMAGE_BYTES / (1024 * 1024)
                )),
            );
        }
        return match fs::read(pb) {
            Ok(bytes) => PreviewPayload {
                kind: "image".into(),
                path: trimmed.to_string(),
                text: None,
                data_url: Some(format!(
                    "data:{};base64,{}",
                    mime_for_ext(&ext),
                    B64.encode(bytes)
                )),
                truncated: false,
                error: None,
                link: None,
                dir: None,
            },
            Err(err) => unsupported(trimmed, false, Some(format!("Failed to read image: {err}"))),
        };
    }

    if is_text_ext(&ext) || ext.is_empty() {
        let mut file = match fs::File::open(pb) {
            Ok(f) => f,
            Err(err) => {
                return unsupported(trimmed, false, Some(format!("Failed to open file: {err}")));
            }
        };
        let truncated = meta.len() > MAX_TEXT_BYTES;
        let mut limited = file.by_ref().take(MAX_TEXT_BYTES);
        let mut buf = Vec::new();
        if let Err(err) = limited.read_to_end(&mut buf) {
            return unsupported(trimmed, false, Some(format!("Failed to read text: {err}")));
        }
        if buf.iter().filter(|b| **b == 0).count() > 8 {
            return unsupported(trimmed, false, Some("Binary file".into()));
        }
        let text = String::from_utf8_lossy(&buf).into_owned();
        return PreviewPayload {
            kind: "text".into(),
            path: trimmed.to_string(),
            text: Some(text),
            data_url: None,
            truncated,
            error: None,
            link: None,
            dir: None,
        };
    }

    unsupported(trimmed, false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_path_unsupported() {
        let p = preview_load(String::new());
        assert_eq!(p.kind, "unsupported");
    }

    #[test]
    fn dir_preview_returns_folder_info_not_error() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let dir = std::env::temp_dir().join(format!("afm-preview-dir-{stamp}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        let p = preview_load(dir.to_string_lossy().into_owned());
        assert_eq!(p.kind, "dir");
        assert!(p.error.is_none());
        let info = p.dir.expect("dir payload carries dir info");
        assert_eq!(info.file_count, Some(1));
        assert_eq!(info.dir_count, Some(1));
        assert!(info.modified_ms.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_preview_reads_file() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = std::env::temp_dir().join(format!("afm-preview-{stamp}.txt"));
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "hello preview").unwrap();
        let p = preview_load(path.to_string_lossy().into_owned());
        assert_eq!(p.kind, "text");
        assert!(p.text.unwrap().contains("hello preview"));
        let _ = fs::remove_file(path);
    }

    #[cfg(windows)]
    #[test]
    fn lnk_preview_returns_link_info() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let dir = std::env::temp_dir().join(format!("afm-preview-lnk-{stamp}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target.txt");
        fs::write(&target, "link me").unwrap();
        let created = crate::file_ops::shortcut::create_shortcuts(
            &[target.to_string_lossy().into_owned()],
            &dir.to_string_lossy(),
            crate::file_ops::NameConflict::KeepBoth,
        )
        .unwrap();
        assert_eq!(created.len(), 1);

        let p = preview_load(created[0].clone());
        assert_eq!(p.kind, "link");
        let link = p.link.expect("link payload carries link info");
        assert_eq!(link.kind, "shortcut");
        let got_target = link.target.expect("lnk target resolved");
        assert!(
            got_target.to_ascii_lowercase().ends_with("target.txt"),
            "unexpected target: {got_target}"
        );
        assert!(link.target_exists);
        assert_eq!(link.target_kind.as_deref(), Some("file"));
        assert!(link.target_dir_exists);
        let got_dir = link.target_dir.expect("target dir (原目录) resolved");
        assert!(
            got_dir.to_ascii_lowercase().contains("afm-preview-lnk"),
            "unexpected target dir: {got_dir}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn broken_junction_preview_returns_link_info_not_error() {
        use std::process::Command;

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let dir = std::env::temp_dir().join(format!("afm-preview-junc-{stamp}"));
        let _ = fs::remove_dir_all(&dir);
        let real = dir.join("real");
        fs::create_dir_all(&real).unwrap();
        let real_s = fs::canonicalize(&real)
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        let link = dir.join("alias");
        let status = Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &real_s,
            ])
            .status()
            .unwrap();
        assert!(status.success());
        fs::remove_dir_all(&real).unwrap();

        let p = preview_load(link.to_string_lossy().into_owned());
        assert_eq!(p.kind, "link");
        assert!(p.error.is_none());
        let info = p.link.expect("broken junction carries link info");
        assert_eq!(info.kind, "symlink");
        assert!(!info.target_exists);
        assert!(info.target.is_some());
        // The link is broken but its 原目录 (temp dir) still exists.
        assert!(info.target_dir_exists);
        let _ = fs::remove_dir_all(&dir);
    }
}
