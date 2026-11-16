// SPDX-License-Identifier: GPL-3.0-or-later
//! Free disk space for the volume containing a path.
//!
//! 7.16: OS-specific probing (`GetDiskFreeSpaceExW` on Windows,
//! `sysinfo::Disks` longest-mount-point match elsewhere) now lives in the
//! platform adapters. This module only maps the neutral `VolumeSpace` onto the
//! frontend `DiskSpaceInfo` wire contract.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpaceInfo {
    pub path: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
}

pub fn disk_space_for_path(path: &str) -> Result<DiskSpaceInfo, String> {
    let space = crate::platform::volume_space(path)?;
    Ok(DiskSpaceInfo {
        path: space.path,
        free_bytes: space.free_bytes,
        total_bytes: space.total_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path_is_rejected() {
        assert!(disk_space_for_path("  ").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn free_space_for_system_drive() {
        let info = disk_space_for_path(r"C:\").unwrap();
        assert!(info.total_bytes > 0);
        assert!(info.free_bytes <= info.total_bytes);
    }

    #[test]
    #[cfg(not(windows))]
    fn free_space_for_root_volume() {
        let info = disk_space_for_path("/").unwrap();
        assert!(info.total_bytes > 0);
        assert!(info.free_bytes <= info.total_bytes);
        assert!(info.path.starts_with('/'));
    }
}
