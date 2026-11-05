// SPDX-License-Identifier: GPL-3.0-or-later

use super::path_guard::assert_os_path;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

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
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
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
    Ok(NavigatorEntryStat {
        name,
        path: pb.to_string_lossy().replace('/', "\\"),
        kind: if is_dir { "dir" } else { "file" }.to_string(),
        size: if meta.is_file() { meta.len() } else { 0 },
        modified_ms: meta.modified().ok().and_then(system_time_ms),
        created_ms: meta.created().ok().and_then(system_time_ms),
        readonly: meta.permissions().readonly(),
    })
}
