// SPDX-License-Identifier: GPL-3.0-or-later

use super::path_guard::assert_os_path;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};
use walkdir::WalkDir;

use crate::preview::{PreviewLinkInfo, describe_link_target, reparse_link_info};

/// Cap directory total-size walk so Properties stays responsive.
const DIR_SIZE_MAX_ENTRIES: usize = 80_000;
const DIR_SIZE_MAX_MS: u128 = 2_500;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorEntryStat {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub modified_ms: Option<u64>,
    pub created_ms: Option<u64>,
    pub readonly: bool,
    pub hidden: bool,
    /// Directory content total (files only); `None` for files or if not computed.
    pub total_size: Option<u64>,
    pub file_count: Option<u64>,
    pub dir_count: Option<u64>,
    /// True when the directory walk stopped early (cap / timeout).
    pub total_size_truncated: bool,
    /// Link metadata (target / 原目录) for `.lnk` shortcuts and reparse points.
    pub link: Option<PreviewLinkInfo>,
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn dir_totals(path: &PathBuf) -> (u64, u64, u64, bool) {
    let started = Instant::now();
    let mut total_size = 0u64;
    let mut file_count = 0u64;
    let mut dir_count = 0u64;
    let mut seen = 0usize;
    let mut truncated = false;

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        seen += 1;
        if seen > DIR_SIZE_MAX_ENTRIES || started.elapsed().as_millis() > DIR_SIZE_MAX_MS {
            truncated = true;
            break;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_file() {
            total_size = total_size.saturating_add(meta.len());
            file_count = file_count.saturating_add(1);
        } else if meta.is_dir() {
            // Skip the root itself in dir_count so it reflects children.
            if entry.depth() > 0 {
                dir_count = dir_count.saturating_add(1);
            }
        }
    }

    (total_size, file_count, dir_count, truncated)
}

/// Link metadata for the 属性 panel: reparse points (junction / symlink) and
/// `.lnk` shortcuts. Regular files and directories return `None`.
fn entry_link_info(pb: &Path) -> Option<PreviewLinkInfo> {
    if let Some(info) = reparse_link_info(pb) {
        return Some(info);
    }
    let is_lnk = pb
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false);
    if !is_lnk {
        return None;
    }
    match crate::file_ops::shortcut::read_shortcut_target(pb) {
        Ok(target) if target.is_empty() => Some(PreviewLinkInfo {
            kind: "shortcut".into(),
            target: None,
            target_exists: false,
            target_kind: None,
            target_dir: None,
            target_dir_exists: false,
        }),
        Ok(target) => Some(describe_link_target("shortcut", Path::new(&target))),
        Err(_) => None,
    }
}

/// Whether a metadata's raw file attributes carry the directory bit
/// (junctions keep it even when their target is gone).
#[cfg(windows)]
fn attr_is_dir(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    meta.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0
}

#[cfg(not(windows))]
fn attr_is_dir(_meta: &fs::Metadata) -> bool {
    false
}

pub fn stat_path(path: &str) -> Result<NavigatorEntryStat, String> {
    assert_os_path(path)?;
    let pb = PathBuf::from(path.trim());

    // Links: attach target metadata (目标 / 原目录) so 属性 can show it.
    let link = entry_link_info(&pb);

    let followed = fs::metadata(&pb);
    let follows_target = followed.is_ok();
    // Broken reparse point: stat the link node itself instead of erroring
    // ("Path does not exist") — the 属性 panel opens with link info.
    let meta = match followed {
        Ok(meta) => meta,
        Err(_) => fs::symlink_metadata(&pb)
            .map_err(|_| format!("Path does not exist: {path}"))?,
    };
    // Junctions keep the directory attribute on the link node; a broken one
    // still reports as a folder so the 属性 panel shows sensible rows.
    let is_dir = if follows_target {
        meta.is_dir()
    } else {
        attr_is_dir(&meta)
    };
    let name = pb
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| pb.to_string_lossy().into_owned());

    // Only walk totals when the entry resolves to a real directory; a broken
    // link must not surface "0 files" from an impossible walk.
    let (total_size, file_count, dir_count, total_size_truncated) = if is_dir && follows_target {
        let (ts, fc, dc, trunc) = dir_totals(&pb);
        (Some(ts), Some(fc), Some(dc), trunc)
    } else {
        (None, None, None, false)
    };
    // Compute before `name` is moved into the struct literal.
    let hidden = super::list::entry_is_hidden(&meta, &name);

    Ok(NavigatorEntryStat {
        name,
        // Keep the host separator: a hardcoded `\` turns POSIX paths into
        // unopenable strings on macOS (see docs/macos-port-research.md 7.14).
        path: crate::paths::display_path(&pb),
        kind: if is_dir { "dir" } else { "file" }.to_string(),
        size: if meta.is_file() { meta.len() } else { 0 },
        modified_ms: meta.modified().ok().and_then(system_time_ms),
        created_ms: meta.created().ok().and_then(system_time_ms),
        readonly: meta.permissions().readonly(),
        hidden,
        total_size,
        file_count,
        dir_count,
        total_size_truncated,
        link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_path_errors() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let missing = std::env::temp_dir().join(format!("afm-stat-missing-{stamp}"));
        assert!(stat_path(missing.to_string_lossy().as_ref()).is_err());
    }

    #[test]
    fn regular_file_has_no_link_info() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = std::env::temp_dir().join(format!("afm-stat-file-{stamp}.txt"));
        fs::write(&path, "stat me").unwrap();
        let stat = stat_path(path.to_string_lossy().as_ref()).unwrap();
        assert!(stat.link.is_none());
        assert_eq!(stat.kind, "file");
        let _ = fs::remove_file(path);
    }

    #[cfg(windows)]
    #[test]
    fn broken_junction_stat_returns_link_info_not_error() {
        use std::process::Command;

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let dir = std::env::temp_dir().join(format!("afm-stat-junc-{stamp}"));
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
            .args(["/C", "mklink", "/J", &link.to_string_lossy(), &real_s])
            .status()
            .unwrap();
        assert!(status.success());
        fs::remove_dir_all(&real).unwrap();

        let stat = stat_path(link.to_string_lossy().as_ref()).unwrap();
        assert_eq!(stat.name, "alias");
        assert_eq!(stat.kind, "dir");
        assert!(stat.total_size.is_none(), "broken link must not walk totals");
        let info = stat.link.expect("broken junction stat carries link info");
        assert_eq!(info.kind, "symlink");
        assert!(!info.target_exists);
        assert!(info.target_dir_exists);
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn lnk_stat_attaches_shortcut_link_info() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let dir = std::env::temp_dir().join(format!("afm-stat-lnk-{stamp}"));
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

        let stat = stat_path(&created[0]).unwrap();
        assert_eq!(stat.kind, "file");
        let info = stat.link.expect("lnk stat carries link info");
        assert_eq!(info.kind, "shortcut");
        assert!(info.target_exists);
        assert_eq!(info.target_kind.as_deref(), Some("file"));
        assert!(info.target_dir_exists);
        let _ = fs::remove_dir_all(&dir);
    }
}
