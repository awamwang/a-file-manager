// SPDX-License-Identifier: GPL-3.0-or-later
//! Free disk space for the volume containing a path.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpaceInfo {
    pub path: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
}

fn volume_root(path: &str) -> Option<String> {
    let trimmed = path.trim().replace('/', "\\");
    if trimmed.starts_with("\\\\") {
        let parts: Vec<&str> = trimmed.trim_start_matches('\\').split('\\').collect();
        if parts.len() >= 2 {
            return Some(format!(r"\\{}\{}\", parts[0], parts[1]));
        }
        return None;
    }
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        let letter = trimmed.chars().next()?.to_ascii_uppercase();
        return Some(format!(r"{letter}:\"));
    }
    None
}

#[cfg(windows)]
pub fn disk_space_for_path(path: &str) -> Result<DiskSpaceInfo, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let root = volume_root(path).ok_or_else(|| "Path has no volume root".to_string())?;
    let wide: Vec<u16> = std::ffi::OsStr::new(&root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free_available = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut free_available),
            Some(&mut total),
            Some(&mut total_free),
        )
        .map_err(|err| format!("GetDiskFreeSpaceExW failed: {err}"))?;
    }
    Ok(DiskSpaceInfo {
        path: root,
        free_bytes: free_available,
        total_bytes: total,
    })
}

#[cfg(not(windows))]
pub fn disk_space_for_path(path: &str) -> Result<DiskSpaceInfo, String> {
    let _ = volume_root(path);
    Err("Disk space query is only implemented on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_root_drive_and_unc() {
        assert_eq!(volume_root(r"C:\Temp\afm-fixture\a").as_deref(), Some(r"C:\"));
        assert_eq!(
            volume_root(r"\\server\share\folder").as_deref(),
            Some(r"\\server\share\")
        );
        assert_eq!(volume_root("afm://guide"), None);
    }

    #[test]
    #[cfg(windows)]
    fn free_space_for_system_drive() {
        let info = disk_space_for_path(r"C:\").unwrap();
        assert!(info.total_bytes > 0);
        assert!(info.free_bytes <= info.total_bytes);
    }
}
