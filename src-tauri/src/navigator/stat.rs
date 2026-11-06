// SPDX-License-Identifier: GPL-3.0-or-later

use super::path_guard::assert_os_path;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};
use walkdir::WalkDir;

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

pub fn stat_path(path: &str) -> Result<NavigatorEntryStat, String> {
    assert_os_path(path)?;
    let pb = PathBuf::from(path.trim());
    if !pb.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    let meta = fs::metadata(&pb).map_err(|err| format!("Failed to read metadata: {err}"))?;
    let is_dir = meta.is_dir();
    let name = pb
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| pb.to_string_lossy().into_owned());

    let (total_size, file_count, dir_count, total_size_truncated) = if is_dir {
        let (ts, fc, dc, trunc) = dir_totals(&pb);
        (Some(ts), Some(fc), Some(dc), trunc)
    } else {
        (None, None, None, false)
    };

    Ok(NavigatorEntryStat {
        name,
        path: pb.to_string_lossy().replace('/', "\\"),
        kind: if is_dir { "dir" } else { "file" }.to_string(),
        size: if meta.is_file() { meta.len() } else { 0 },
        modified_ms: meta.modified().ok().and_then(system_time_ms),
        created_ms: meta.created().ok().and_then(system_time_ms),
        readonly: meta.permissions().readonly(),
        hidden: super::list::entry_is_hidden(&meta),
        total_size,
        file_count,
        dir_count,
        total_size_truncated,
    })
}
